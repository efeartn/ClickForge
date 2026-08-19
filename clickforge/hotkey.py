from typing import Callable, Any, Optional
from pynput import keyboard

class HotkeyManager:
    def __init__(self, hotkey: str, on_toggle: Callable[[], None]):
        self.hotkey = hotkey.lower()
        self._on_toggle = on_toggle
        self._listener: Optional[keyboard.Listener] = None
        self._recording = False
        self._on_recorded: Optional[Callable[[str], None]] = None

    def start(self):
        if self._listener is None:
            self._listener = keyboard.Listener(on_press=self._on_press)
            self._listener.start()

    def stop(self):
        if self._listener:
            self._listener.stop()
            self._listener = None

    def set_hotkey(self, hotkey: str):
        self.hotkey = hotkey.lower()

    def start_recording(self, callback: Callable[[str], None]):
        self._on_recorded = callback
        self._recording = True

    def _key_to_string(self, key: Any) -> str:
        if isinstance(key, keyboard.Key):
            return key.name.lower()
        elif hasattr(key, 'char') and key.char is not None:
            return key.char.lower()
        else:
            return str(key).lower()

    def _on_press(self, key: Any):
        key_str = self._key_to_string(key)
        if self._recording:
            self._recording = False
            if self._on_recorded:
                self._on_recorded(key_str)
        else:
            if key_str == self.hotkey:
                if self._on_toggle:
                    self._on_toggle()
