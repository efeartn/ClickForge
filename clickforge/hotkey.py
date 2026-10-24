"""Re-exports from :mod:`clickforge.core.hotkeys` for backward compatibility."""

from clickforge.core.hotkeys import (
    PANIC_KEY,
    TriggerListener,
    display_name,
    is_mouse_binding,
)

HotkeyManager = TriggerListener

__all__ = [
    "HotkeyManager",
    "PANIC_KEY",
    "TriggerListener",
    "display_name",
    "is_mouse_binding",
]
