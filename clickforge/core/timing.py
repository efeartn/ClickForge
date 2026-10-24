"""High-precision timing primitives.

Windows' default scheduler tick is ~15.6 ms, so ``time.sleep`` and
``Event.wait`` overshoot badly for short intervals.  We solve it in two parts:

1. :class:`HighResolutionTimer` requests 1 ms timer resolution via
   ``timeBeginPeriod(1)`` only while the clicker is running, and always
   releases it with ``timeEndPeriod(1)``.
2. :func:`wait_until` sleeps for the coarse part of the interval through an
   interruptible ``Event.wait`` and then spin-waits on ``perf_counter`` for the
   final ~1.5 ms.
"""

from __future__ import annotations

import sys
import threading
import time

IS_WINDOWS = sys.platform.startswith("win")

#: Portion of every wait that is spin-waited instead of slept (seconds).
SPIN_THRESHOLD = 0.0015

if IS_WINDOWS:
    import ctypes

    _winmm = ctypes.WinDLL("winmm")
    _winmm.timeBeginPeriod.argtypes = (ctypes.c_uint,)
    _winmm.timeEndPeriod.argtypes = (ctypes.c_uint,)


class HighResolutionTimer:
    """Context manager that raises the OS timer resolution to 1 ms.

    A no-op on non-Windows platforms (their schedulers are already precise).
    """

    def __init__(self, period_ms: int = 1) -> None:
        self._period = period_ms
        self._active = False

    def __enter__(self) -> "HighResolutionTimer":
        if IS_WINDOWS:
            try:
                self._active = _winmm.timeBeginPeriod(self._period) == 0
            except OSError:
                self._active = False
        return self

    def __exit__(self, *exc) -> None:
        if self._active:
            try:
                _winmm.timeEndPeriod(self._period)
            finally:
                self._active = False


def wait_until(deadline: float, stop_event: threading.Event) -> bool:
    """Block until ``time.perf_counter() >= deadline``.

    Returns ``False`` as soon as ``stop_event`` is set, ``True`` otherwise.
    """
    perf = time.perf_counter
    while True:
        remaining = deadline - perf()
        if remaining <= 0:
            return not stop_event.is_set()
        if remaining > SPIN_THRESHOLD:
            if stop_event.wait(remaining - SPIN_THRESHOLD):
                return False
            continue
        # Final stretch: light spin.  sleep(0) releases the GIL so the UI and
        # hook threads stay responsive.
        while perf() < deadline:
            if stop_event.is_set():
                return False
            time.sleep(0)
        return not stop_event.is_set()


def wait_for(seconds: float, stop_event: threading.Event) -> bool:
    """Relative variant of :func:`wait_until`."""
    if seconds <= 0:
        return not stop_event.is_set()
    return wait_until(time.perf_counter() + seconds, stop_event)
