"""Low-latency mouse input injection.

On Windows we talk to ``user32.SendInput`` directly through pre-allocated
``INPUT`` structures, which avoids per-click object churn in high-level
wrappers.  Every other platform falls back to ``pynput``.
"""

from __future__ import annotations

import sys
from typing import Dict, Optional, Tuple

IS_WINDOWS = sys.platform.startswith("win")

BUTTONS = ("left", "right", "middle")

#: Tag written to ``dwExtraInfo`` so injected events can be identified.
INJECTED_MARKER = 0x434C4B46  # "CLKF"


class InputBackend:
    """Abstract mouse backend."""

    def press(self, button: str) -> None:
        raise NotImplementedError

    def release(self, button: str) -> None:
        raise NotImplementedError

    def click(self, button: str) -> None:
        """Press and release with zero hold time."""
        self.press(button)
        self.release(button)

    def move(self, x: int, y: int) -> None:
        raise NotImplementedError

    def position(self) -> Optional[Tuple[int, int]]:
        raise NotImplementedError


if IS_WINDOWS:
    import ctypes
    from ctypes import wintypes

    ULONG_PTR = ctypes.c_size_t

    class MOUSEINPUT(ctypes.Structure):
        _fields_ = [
            ("dx", wintypes.LONG),
            ("dy", wintypes.LONG),
            ("mouseData", wintypes.DWORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class KEYBDINPUT(ctypes.Structure):
        _fields_ = [
            ("wVk", wintypes.WORD),
            ("wScan", wintypes.WORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class HARDWAREINPUT(ctypes.Structure):
        _fields_ = [
            ("uMsg", wintypes.DWORD),
            ("wParamL", wintypes.WORD),
            ("wParamH", wintypes.WORD),
        ]

    class _INPUTUNION(ctypes.Union):
        _fields_ = [("mi", MOUSEINPUT), ("ki", KEYBDINPUT), ("hi", HARDWAREINPUT)]

    class INPUT(ctypes.Structure):
        _anonymous_ = ("u",)
        _fields_ = [("type", wintypes.DWORD), ("u", _INPUTUNION)]

    INPUT_MOUSE = 0
    _FLAGS = {
        "left": (0x0002, 0x0004),
        "right": (0x0008, 0x0010),
        "middle": (0x0020, 0x0040),
    }

    # Private DLL handle so our argtypes never clash with pynput's.
    _user32 = ctypes.WinDLL("user32", use_last_error=True)
    _user32.SendInput.argtypes = (wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int)
    _user32.SendInput.restype = wintypes.UINT
    _user32.SetCursorPos.argtypes = (ctypes.c_int, ctypes.c_int)
    _user32.SetCursorPos.restype = wintypes.BOOL
    _user32.GetCursorPos.argtypes = (ctypes.POINTER(wintypes.POINT),)
    _user32.GetCursorPos.restype = wintypes.BOOL

    def _mouse_input(flags: int) -> "INPUT":
        inp = INPUT()
        inp.type = INPUT_MOUSE
        inp.mi = MOUSEINPUT(0, 0, 0, flags, 0, INJECTED_MARKER)
        return inp

    class Win32Backend(InputBackend):
        """``SendInput`` backend with pre-built input arrays per button."""

        def __init__(self) -> None:
            self._size = ctypes.sizeof(INPUT)
            self._down: Dict[str, ctypes.Array] = {}
            self._up: Dict[str, ctypes.Array] = {}
            self._click: Dict[str, ctypes.Array] = {}
            for name, (down, up) in _FLAGS.items():
                self._down[name] = (INPUT * 1)(_mouse_input(down))
                self._up[name] = (INPUT * 1)(_mouse_input(up))
                self._click[name] = (INPUT * 2)(_mouse_input(down), _mouse_input(up))
            self._point = wintypes.POINT()

        def press(self, button: str) -> None:
            _user32.SendInput(1, self._down[button], self._size)

        def release(self, button: str) -> None:
            _user32.SendInput(1, self._up[button], self._size)

        def click(self, button: str) -> None:
            _user32.SendInput(2, self._click[button], self._size)

        def move(self, x: int, y: int) -> None:
            _user32.SetCursorPos(int(x), int(y))

        def position(self) -> Optional[Tuple[int, int]]:
            if _user32.GetCursorPos(ctypes.byref(self._point)):
                return self._point.x, self._point.y
            return None


class PynputBackend(InputBackend):
    """Portable fallback for macOS and Linux (X11)."""

    def __init__(self) -> None:
        from pynput.mouse import Button, Controller

        self._mouse = Controller()
        self._buttons = {"left": Button.left, "right": Button.right, "middle": Button.middle}

    def press(self, button: str) -> None:
        self._mouse.press(self._buttons[button])

    def release(self, button: str) -> None:
        self._mouse.release(self._buttons[button])

    def move(self, x: int, y: int) -> None:
        self._mouse.position = (int(x), int(y))

    def position(self) -> Optional[Tuple[int, int]]:
        try:
            x, y = self._mouse.position
            return int(x), int(y)
        except Exception:
            return None


def create_backend() -> InputBackend:
    if IS_WINDOWS:
        return Win32Backend()
    return PynputBackend()
