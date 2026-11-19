"""OS integration: DPI awareness, taskbar identity, screen geometry, assets.

:func:`prepare_process` must run before any Tk window is created.
"""

from __future__ import annotations

import os
import sys
from typing import List, Optional, Tuple

IS_WINDOWS = sys.platform.startswith("win")
IS_MAC = sys.platform == "darwin"
APP_USER_MODEL_ID = "clickforge.app"


def resource_path(*parts: str) -> str:
    """Absolute path to a bundled resource (works from source and PyInstaller)."""
    base = getattr(sys, "_MEIPASS", None)
    if base is None:
        base = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    return os.path.join(base, *parts)


def enable_dpi_awareness() -> None:
    """Opt into Per-Monitor (v2) DPI awareness on Windows.

    Without this, Windows virtualizes coordinates on scaled displays (125 %,
    150 %, ...) and picked points no longer match where clicks land.
    """
    if not IS_WINDOWS:
        return
    import ctypes

    try:
        user32 = ctypes.WinDLL("user32")
        fn = user32.SetProcessDpiAwarenessContext
        fn.argtypes = (ctypes.c_void_p,)
        fn.restype = ctypes.c_int
        if fn(ctypes.c_void_p(-4)):  # DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
            return
    except (AttributeError, OSError):
        pass
    try:
        ctypes.WinDLL("shcore").SetProcessDpiAwareness(2)  # PROCESS_PER_MONITOR_DPI_AWARE
        return
    except (AttributeError, OSError):
        pass
    try:
        ctypes.WinDLL("user32").SetProcessDPIAware()
    except (AttributeError, OSError):
        pass


def set_app_user_model_id() -> None:
    """Give the process its own taskbar identity so Windows shows our icon."""
    if not IS_WINDOWS:
        return
    import ctypes

    try:
        ctypes.WinDLL("shell32").SetCurrentProcessExplicitAppUserModelID(APP_USER_MODEL_ID)
    except (AttributeError, OSError):
        pass


def prepare_process() -> None:
    enable_dpi_awareness()
    set_app_user_model_id()


def virtual_screen_bounds() -> Optional[Tuple[int, int, int, int]]:
    """``(left, top, width, height)`` of the whole multi-monitor desktop."""
    if IS_WINDOWS:
        import ctypes

        gsm = ctypes.WinDLL("user32").GetSystemMetrics
        w, h = gsm(78), gsm(79)
        if w <= 0 or h <= 0:
            return None
        return gsm(76), gsm(77), w, h

    if IS_MAC:
        import ctypes

        class _CGPoint(ctypes.Structure):
            _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]

        class _CGSize(ctypes.Structure):
            _fields_ = [("width", ctypes.c_double), ("height", ctypes.c_double)]

        class _CGRect(ctypes.Structure):
            _fields_ = [("origin", _CGPoint), ("size", _CGSize)]

        try:
            cg = ctypes.cdll.LoadLibrary(
                "/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics"
            )
            cg.CGGetActiveDisplayList.argtypes = (
                ctypes.c_uint32,
                ctypes.POINTER(ctypes.c_uint32),
                ctypes.POINTER(ctypes.c_uint32),
            )
            cg.CGGetActiveDisplayList.restype = ctypes.c_int32
            cg.CGDisplayBounds.argtypes = (ctypes.c_uint32,)
            cg.CGDisplayBounds.restype = _CGRect

            count = ctypes.c_uint32(0)
            if cg.CGGetActiveDisplayList(0, None, ctypes.byref(count)) != 0 or count.value == 0:
                return None
            displays = (ctypes.c_uint32 * count.value)()
            if cg.CGGetActiveDisplayList(count.value, displays, ctypes.byref(count)) != 0:
                return None
            rects = [cg.CGDisplayBounds(displays[i]) for i in range(count.value)]
            left = min(r.origin.x for r in rects)
            top = min(r.origin.y for r in rects)
            right = max(r.origin.x + r.size.width for r in rects)
            bottom = max(r.origin.y + r.size.height for r in rects)
            w = int(round(right - left))
            h = int(round(bottom - top))
            if w <= 0 or h <= 0:
                return None
            return int(round(left)), int(round(top)), w, h
        except (OSError, AttributeError):
            return None

    return None


def startup_warnings() -> List[str]:
    """Platform issues the user should know about before clicking."""
    warnings: List[str] = []
    if IS_MAC:
        try:
            import ctypes

            lib = ctypes.cdll.LoadLibrary(
                "/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices"
            )
            lib.AXIsProcessTrusted.restype = ctypes.c_bool
            if not lib.AXIsProcessTrusted():
                warnings.append(
                    "ClickForge needs Accessibility access to send clicks and read hotkeys.\n"
                    "Open System Settings > Privacy & Security > Accessibility and enable it."
                )
        except OSError:
            pass
    elif sys.platform.startswith("linux"):
        if os.environ.get("XDG_SESSION_TYPE", "").lower() == "wayland":
            warnings.append(
                "You are running a Wayland session. Global hotkeys and synthetic clicks only "
                "work in X11/XWayland applications."
            )
    return warnings
