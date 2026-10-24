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
    if not IS_WINDOWS:
        return None
    import ctypes

    gsm = ctypes.WinDLL("user32").GetSystemMetrics
    w, h = gsm(78), gsm(79)
    if w <= 0 or h <= 0:
        return None
    return gsm(76), gsm(77), w, h


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
