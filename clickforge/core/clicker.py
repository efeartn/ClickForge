"""Click engine.

The engine runs on a dedicated worker thread and never touches the UI.  The
UI polls :attr:`ClickEngine.click_count` and :meth:`ClickEngine.cps` at a
fixed rate instead.

Settings are passed as an immutable :class:`ClickSettings` snapshot when a
run starts, so editing the UI mid-run can never corrupt the loop's state.
"""

from __future__ import annotations

import collections
import math
import random
import threading
import time
from dataclasses import dataclass
from typing import Callable, Optional, Tuple

from clickforge.core.input_backend import BUTTONS, InputBackend, create_backend
from clickforge.core.timing import HighResolutionTimer, wait_for, wait_until

MIN_INTERVAL_MS = 1.0
MAX_INTERVAL_MS = 3_600_000.0
#: Cursor within this many pixels of the primary screen's top-left aborts a run.
CORNER_FAILSAFE_PX = 2


@dataclass(frozen=True)
class ClickSettings:
    """Immutable snapshot of everything the worker needs."""

    interval_ms: float = 100.0
    button: str = "left"
    double: bool = False
    #: Number of click actions to perform; 0 means run until stopped.
    repeat_count: int = 0
    #: ``None`` follows the cursor; otherwise clicks land at this point.
    fixed_position: Optional[Tuple[int, int]] = None
    humanize: bool = False
    #: Gaussian sigma for interval jitter, as a percentage of the interval.
    jitter_pct: float = 15.0
    hold_min_ms: float = 18.0
    hold_max_ms: float = 42.0
    position_jitter_px: int = 0
    corner_failsafe: bool = True

    def sanitized(self) -> "ClickSettings":
        """Return a copy with every field clamped to a safe range."""

        def finite(v: float, default: float) -> float:
            try:
                v = float(v)
            except (TypeError, ValueError):
                return default
            return v if math.isfinite(v) else default

        hold_lo = min(max(finite(self.hold_min_ms, 18.0), 0.0), 500.0)
        hold_hi = min(max(finite(self.hold_max_ms, 42.0), 0.0), 500.0)
        if hold_lo > hold_hi:
            hold_lo, hold_hi = hold_hi, hold_lo
        return ClickSettings(
            interval_ms=min(max(finite(self.interval_ms, 100.0), MIN_INTERVAL_MS), MAX_INTERVAL_MS),
            button=self.button if self.button in BUTTONS else "left",
            double=bool(self.double),
            repeat_count=max(0, int(self.repeat_count or 0)),
            fixed_position=(
                (int(self.fixed_position[0]), int(self.fixed_position[1]))
                if self.fixed_position is not None
                else None
            ),
            humanize=bool(self.humanize),
            jitter_pct=min(max(finite(self.jitter_pct, 15.0), 0.0), 50.0),
            hold_min_ms=hold_lo,
            hold_max_ms=hold_hi,
            position_jitter_px=min(max(int(self.position_jitter_px or 0), 0), 50),
            corner_failsafe=bool(self.corner_failsafe),
        )


class ClickEngine:
    """Thread-based auto-click engine with guaranteed button release."""

    def __init__(
        self,
        backend: Optional[InputBackend] = None,
        on_stopped: Optional[Callable[[str], None]] = None,
    ) -> None:
        self._backend = backend or create_backend()
        self._on_stopped = on_stopped
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._control_lock = threading.Lock()
        self._rng = random.Random()

        # Written only by the worker; plain int reads are atomic under the GIL.
        self._click_count = 0
        # Worker appends on the right, the UI prunes from the left.  deque
        # append/popleft are thread-safe in CPython, so no lock is needed.
        self._timestamps: collections.deque = collections.deque(maxlen=8192)
        # Button currently held down by the worker (for the failsafe).
        self._held: Optional[str] = None

    # ── Public API ───────────────────────────────────────────────────

    @property
    def backend(self) -> InputBackend:
        return self._backend

    @property
    def is_running(self) -> bool:
        t = self._thread
        return t is not None and t.is_alive()

    @property
    def click_count(self) -> int:
        return self._click_count

    def cps(self) -> int:
        """Physical clicks performed during the last rolling second."""
        cutoff = time.perf_counter() - 1.0
        ts = self._timestamps
        try:
            while ts and ts[0] < cutoff:
                ts.popleft()
        except IndexError:
            pass
        return len(ts)

    def reset_counter(self) -> None:
        self._click_count = 0
        self._timestamps.clear()

    def start(self, settings: ClickSettings) -> bool:
        """Start a run.  Returns ``False`` if a run is already active."""
        with self._control_lock:
            if self.is_running:
                return False
            settings = settings.sanitized()
            self._stop_event = threading.Event()
            self._thread = threading.Thread(
                target=self._run,
                args=(settings, self._stop_event),
                name="ClickForgeWorker",
                daemon=True,
            )
            self._thread.start()
            return True

    def stop(self, wait: bool = False, timeout: float = 1.0) -> None:
        """Request the worker to stop.  Safe to call from any thread."""
        self._stop_event.set()
        if wait:
            t = self._thread
            if t is not None and t is not threading.current_thread():
                t.join(timeout)
            # Last-resort failsafe if the worker could not finish in time.
            self.release_held()

    def release_held(self) -> None:
        held = self._held
        if held is not None:
            try:
                self._backend.release(held)
            finally:
                self._held = None

    # ── Worker ───────────────────────────────────────────────────────

    def _run(self, s: ClickSettings, stop: threading.Event) -> None:
        backend = self._backend
        rng = self._rng
        perf = time.perf_counter
        period = s.interval_ms / 1000.0
        sigma = period * s.jitter_pct / 100.0 if s.humanize else 0.0
        use_hold = s.humanize and s.hold_max_ms > 0
        # Never hold longer than 60% of the interval, or the cadence breaks.
        hold_cap = period * 0.6
        clicks_per_action = 2 if s.double else 1
        remaining = s.repeat_count
        reason = "stopped"

        try:
            with HighResolutionTimer():
                next_deadline = perf()
                while not stop.is_set():
                    if s.corner_failsafe and s.fixed_position is None and self._in_corner():
                        reason = "failsafe"
                        break

                    if s.fixed_position is not None:
                        x, y = s.fixed_position
                        if s.humanize and s.position_jitter_px:
                            x, y = self._jitter_point(x, y, s.position_jitter_px)
                        backend.move(x, y)

                    for i in range(clicks_per_action):
                        if use_hold:
                            hold = min(
                                rng.triangular(s.hold_min_ms, s.hold_max_ms) / 1000.0,
                                hold_cap / clicks_per_action,
                            )
                            self._held = s.button
                            backend.press(s.button)
                            wait_for(hold, stop)  # release happens even if stopped
                            backend.release(s.button)
                            self._held = None
                        else:
                            backend.click(s.button)
                        self._click_count += 1
                        self._timestamps.append(perf())
                        if i + 1 < clicks_per_action and use_hold:
                            # Natural gap between the two halves of a double click.
                            if not wait_for(min(hold, hold_cap / 2), stop):
                                break

                    if remaining:
                        remaining -= 1
                        if remaining == 0:
                            reason = "completed"
                            break

                    delay = period
                    if sigma:
                        noise = rng.gauss(0.0, sigma)
                        noise = max(-2.5 * sigma, min(2.5 * sigma, noise))
                        delay = max(MIN_INTERVAL_MS / 1000.0, period + noise)
                    next_deadline += delay
                    now = perf()
                    if next_deadline < now - period:
                        # We fell far behind (system stall); don't burst-click to catch up.
                        next_deadline = now
                    if not wait_until(next_deadline, stop):
                        break
        except Exception:
            reason = "error"
        finally:
            # Guaranteed mouse-up: a stop can never leave a button stuck down.
            self.release_held()
            if self._on_stopped is not None:
                try:
                    self._on_stopped(reason)
                except Exception:
                    pass

    def _jitter_point(self, x: int, y: int, radius: int) -> Tuple[int, int]:
        # Uniform distribution over a disk.
        r = radius * math.sqrt(self._rng.random())
        a = self._rng.uniform(0.0, math.tau)
        return int(round(x + r * math.cos(a))), int(round(y + r * math.sin(a)))

    def _in_corner(self) -> bool:
        pos = self._backend.position()
        return pos is not None and pos[0] <= CORNER_FAILSAFE_PX and pos[1] <= CORNER_FAILSAFE_PX
