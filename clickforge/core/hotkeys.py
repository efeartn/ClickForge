"""Global trigger hotkeys (keyboard keys and Mouse4/Mouse5).

The listener only reports events through callbacks; it never touches the UI.
Bindings are stored as strings:

* keyboard keys use pynput names (``"f6"``, ``"space"``, ``"a"``, ``"vk96"``)
* side mouse buttons use ``"mouse:x1"`` (Mouse4) and ``"mouse:x2"`` (Mouse5)

The mouse hook is only installed while it is needed (a mouse binding is
active or a new binding is being recorded), so idle overhead stays at zero.
"""

from __future__ import annotations

import re
import threading
from typing import Any, Callable, Optional

from pynput import keyboard, mouse

PANIC_KEY = "esc"

_MOUSE_NAMES = {
    "x1": "mouse:x1",
    "x2": "mouse:x2",
    "button8": "mouse:x1",  # X11 naming
    "button9": "mouse:x2",
}
_DISPLAY = {"mouse:x1": "Mouse4", "mouse:x2": "Mouse5"}


def display_name(binding: str) -> str:
    """Human-readable label for a binding string."""
    if binding in _DISPLAY:
        return _DISPLAY[binding]
    if re.fullmatch(r"f\d{1,2}", binding):
        return binding.upper()
    if len(binding) == 1 or binding.startswith("vk"):
        return binding.upper()
    return binding.replace("_", " ").title()


def is_mouse_binding(binding: str) -> bool:
    return binding.startswith("mouse:")


class TriggerListener:
    """Listens for the trigger binding plus the global panic key.

    Callbacks run on pynput's listener threads; the consumer must hand off to
    its own thread (e.g. a queue polled by the UI) where required.
    """

    def __init__(
        self,
        binding: str,
        on_press: Callable[[], None],
        on_release: Callable[[], None],
        on_panic: Callable[[], None],
        on_recorded: Callable[[Optional[str]], None],
    ) -> None:
        self._binding = binding
        self._on_press = on_press
        self._on_release = on_release
        self._on_panic = on_panic
        self._on_recorded = on_recorded
        self._kb: Optional[keyboard.Listener] = None
        self._mouse: Optional[mouse.Listener] = None
        self._lock = threading.Lock()
        self._mouse_lock = threading.Lock()
        self._recording = False
        self._suspended = False
        self._down = False

    # ── Lifecycle ────────────────────────────────────────────────────

    def start(self) -> None:
        if self._kb is None:
            self._kb = keyboard.Listener(on_press=self._key_press, on_release=self._key_release)
            self._kb.daemon = True
            self._kb.start()
        self._sync_mouse_listener()

    def stop(self) -> None:
        if self._kb is not None:
            self._kb.stop()
            self._kb = None
        if self._mouse is not None:
            self._mouse.stop()
            self._mouse = None

    # ── State ────────────────────────────────────────────────────────

    @property
    def binding(self) -> str:
        return self._binding

    @property
    def trigger_down(self) -> bool:
        return self._down

    def set_binding(self, binding: str) -> None:
        with self._lock:
            self._binding = binding
            self._down = False
        self._sync_mouse_listener()

    def set_suspended(self, suspended: bool) -> None:
        """While suspended the trigger is ignored (panic still works)."""
        self._suspended = suspended
        if suspended:
            self._down = False

    def begin_recording(self) -> None:
        self._recording = True
        self._down = False
        self._sync_mouse_listener()

    def cancel_recording(self) -> None:
        self._recording = False
        self._sync_mouse_listener()

    # ── Internals ────────────────────────────────────────────────────

    def _sync_mouse_listener(self) -> None:
        with self._mouse_lock:
            need = self._kb is not None and (self._recording or is_mouse_binding(self._binding))
            if need and self._mouse is None:
                self._mouse = mouse.Listener(on_click=self._mouse_click)
                self._mouse.daemon = True
                self._mouse.start()
            elif not need and self._mouse is not None:
                self._mouse.stop()
                self._mouse = None

    def _key_name(self, key: Any) -> Optional[str]:
        if self._kb is not None and hasattr(self._kb, "canonical"):
            try:
                key = self._kb.canonical(key)
            except Exception:
                pass
        if isinstance(key, keyboard.Key):
            return key.name
        if isinstance(key, keyboard.KeyCode):
            if key.char and len(key.char) == 1 and key.char.isprintable():
                return key.char.lower()
            if key.vk is not None:
                return f"vk{key.vk}"
        return None

    def _finish_recording(self, name: Optional[str]) -> None:
        self._recording = False
        self._sync_mouse_listener_async()
        self._on_recorded(name)

    def _sync_mouse_listener_async(self) -> None:
        # pynput forbids stopping a listener from inside its own callback.
        threading.Thread(target=self._sync_mouse_listener, daemon=True).start()

    def _trigger(self, pressed: bool) -> None:
        if pressed:
            if self._down or self._suspended:
                return  # ignore OS key auto-repeat
            self._down = True
            self._on_press()
        else:
            if not self._down:
                return
            self._down = False
            self._on_release()

    def _key_press(self, key: Any) -> None:
        name = self._key_name(key)
        if name is None:
            return
        if self._recording:
            self._finish_recording(None if name == PANIC_KEY else name)
            return
        if name == PANIC_KEY:
            self._on_panic()
            return
        if name == self._binding:
            self._trigger(True)

    def _key_release(self, key: Any) -> None:
        name = self._key_name(key)
        if name is not None and name == self._binding and not self._recording:
            self._trigger(False)

    def _mouse_click(self, x: int, y: int, button: Any, pressed: bool) -> None:
        name = _MOUSE_NAMES.get(getattr(button, "name", ""))
        if name is None:
            return  # left/right/middle can never be a trigger
        if self._recording:
            if pressed:
                self._finish_recording(name)
            return
        if name == self._binding:
            self._trigger(pressed)
