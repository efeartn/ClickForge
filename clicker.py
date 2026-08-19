"""
Click engine for ClickForge.

This module contains the ClickEngine class, which performs mouse clicks
in a background thread. It uses native ctypes on Windows for maximum speed
(200+ CPS) and falls back to pynput on macOS/Linux.
"""

import threading
import time
import sys
from typing import Optional, Callable, Tuple
from pynput.mouse import Button, Controller as MouseController

# --- Windows Fast Clicks Setup ---
IS_WINDOWS = sys.platform.startswith('win')
if IS_WINDOWS:
    import ctypes
    from ctypes import wintypes
    PUL = ctypes.POINTER(ctypes.c_ulong)

    class KeyBdInput(ctypes.Structure):
        _fields_ = [("wVk", ctypes.c_ushort), ("wScan", ctypes.c_ushort), ("dwFlags", ctypes.c_ulong), ("time", ctypes.c_ulong), ("dwExtraInfo", PUL)]
    class HardwareInput(ctypes.Structure):
        _fields_ = [("uMsg", ctypes.c_ulong), ("wParamL", ctypes.c_short), ("wParamH", ctypes.c_ushort)]
    class MouseInput(ctypes.Structure):
        _fields_ = [("dx", ctypes.c_long), ("dy", ctypes.c_long), ("mouseData", ctypes.c_ulong), ("dwFlags", ctypes.c_ulong), ("time", ctypes.c_ulong), ("dwExtraInfo", PUL)]
    class Input_I(ctypes.Union):
        _fields_ = [("ki", KeyBdInput), ("mi", MouseInput), ("hi", HardwareInput)]
    class Input(ctypes.Structure):
        _fields_ = [("type", ctypes.c_ulong), ("ii", Input_I)]

    # Windows Mouse Event Flags
    MOUSEEVENTF_LEFTDOWN = 0x0002
    MOUSEEVENTF_LEFTUP = 0x0004
    MOUSEEVENTF_RIGHTDOWN = 0x0008
    MOUSEEVENTF_RIGHTUP = 0x0010
    MOUSEEVENTF_MIDDLEDOWN = 0x0020
    MOUSEEVENTF_MIDDLEUP = 0x0040

    def windows_fast_click(button_str: str, double_click: bool = False):
        extra = ctypes.pointer(ctypes.c_ulong(0))
        if button_str == 'right':
            down, up = MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP
        elif button_str == 'middle':
            down, up = MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP
        else:
            down, up = MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP

        ii_down = Input_I()
        ii_down.mi = MouseInput(0, 0, 0, down, 0, extra)
        x_down = Input(ctypes.c_ulong(0), ii_down)
        
        ii_up = Input_I()
        ii_up.mi = MouseInput(0, 0, 0, up, 0, extra)
        x_up = Input(ctypes.c_ulong(0), ii_up)
        
        inputs = (Input * 2)(x_down, x_up)
        p_inputs = ctypes.pointer(inputs)
        size = ctypes.sizeof(Input)
        
        clicks = 2 if double_click else 1
        for _ in range(clicks):
            ctypes.windll.user32.SendInput(2, p_inputs, size)


class ClickEngine:
    def __init__(
        self,
        on_click: Optional[Callable[[int], None]] = None,
        on_stopped: Optional[Callable[[], None]] = None,
    ) -> None:
        self._mouse = MouseController()
        self._thread: Optional[threading.Thread] = None

        self._stop_event = threading.Event()
        self._click_count: int = 0
        self._lock = threading.Lock()

        self._on_click = on_click
        self._on_stopped = on_stopped

        self.interval_ms: float = 100.0
        self.button_str: str = "left"
        self.pynput_button: Button = Button.left
        self.click_type: str = "single"
        self.click_mode: str = "continuous"
        self.target_count: int = 10
        self.position_mode: str = "cursor"
        self.fixed_position: Tuple[int, int] = (0, 0)

    def configure(
        self,
        interval_ms: float,
        button_str: str,
        click_type: str,
        click_mode: str,
        target_count: int,
        position_mode: str,
        fixed_position: Tuple[int, int],
    ) -> None:
        if self.is_running:
            return

        # Changed to support float MS for ultimate precision (e.g. 0.5 ms)
        self.interval_ms = max(0.1, min(3_600_000.0, float(interval_ms))) 
        self.button_str = button_str
        button_map = {"left": Button.left, "right": Button.right, "middle": Button.middle}
        self.pynput_button = button_map.get(button_str, Button.left)
        self.click_type = click_type
        self.click_mode = click_mode
        self.target_count = max(1, target_count)
        self.position_mode = position_mode
        self.fixed_position = fixed_position

    def start(self) -> None:
        if self.is_running:
            return

        self._stop_event.clear()
        self.reset_counter()

        self._thread = threading.Thread(target=self._click_loop, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        self._stop_event.set()

    def _click_loop(self) -> None:
        # Optimization: Pre-evaluate conditions to avoid loop overhead
        is_fixed_pos = (self.position_mode == "fixed")
        is_fixed_count = (self.click_mode == "fixed_count")
        is_double = (self.click_type == "double")
        interval_sec = self.interval_ms / 1000.0

        while not self._stop_event.is_set():
            if is_fixed_pos:
                try:
                    self._mouse.position = self.fixed_position
                except Exception:
                    pass

            # Fast clicking
            try:
                if IS_WINDOWS:
                    windows_fast_click(self.button_str, is_double)
                else:
                    clicks = 2 if is_double else 1
                    self._mouse.click(self.pynput_button, clicks)
            except Exception:
                pass

            # Update count
            with self._lock:
                self._click_count += 1
                current_count = self._click_count

            if self._on_click:
                self._on_click(current_count)

            if is_fixed_count and current_count >= self.target_count:
                self._stop_event.set()
                if self._on_stopped:
                    self._on_stopped()
                break

            self._precise_wait(interval_sec)

    def _precise_wait(self, seconds: float) -> None:
        if seconds <= 0:
            return
        target_time = time.perf_counter() + seconds

        # BUGFIX: Windows Event.wait() can overshoot by up to ~15.6ms due to the OS tick resolution.
        # If we only subtracted 0.002, wait() could easily overshoot target_time for small delays.
        # We now use a generous 20ms (0.020s) margin so wait() never overshoots.
        # The remainder is strictly spin-waited for perfect microsecond precision!
        if seconds > 0.020:
            sleep_duration = seconds - 0.020
            if self._stop_event.wait(sleep_duration):
                return
        
        # Spin-wait the remaining <20ms
        while time.perf_counter() < target_time:
            if self._stop_event.is_set():
                return

    def reset_counter(self) -> None:
        with self._lock:
            self._click_count = 0

    @property
    def is_running(self) -> bool:
        return self._thread is not None and self._thread.is_alive() and not self._stop_event.is_set()

    @property
    def click_count(self) -> int:
        with self._lock:
            return self._click_count
