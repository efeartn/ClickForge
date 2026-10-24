"""Platform helpers re-exported from :mod:`clickforge.core.system`."""

from __future__ import annotations

import sys
from tkinter import messagebox

from clickforge import __app_name__
from clickforge.core.system import (
    enable_dpi_awareness,
    prepare_process,
    resource_path,
    set_app_user_model_id,
    startup_warnings,
    virtual_screen_bounds,
)


def get_platform() -> str:
    if sys.platform.startswith("win"):
        return "windows"
    if sys.platform == "darwin":
        return "macos"
    return "linux"


def show_platform_warnings(root) -> None:
    for msg in startup_warnings():
        messagebox.showwarning(f"{__app_name__} — Platform Notice", msg, parent=root)


__all__ = [
    "enable_dpi_awareness",
    "get_platform",
    "prepare_process",
    "resource_path",
    "set_app_user_model_id",
    "show_platform_warnings",
    "startup_warnings",
    "virtual_screen_bounds",
]
