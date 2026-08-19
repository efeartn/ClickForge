import threading
from typing import Optional, Callable, Set, Any
from pynput import keyboard

class HotkeyManager:
    """
    Manages global hotkey listening using pynput.
    """
    def __init__(self, hotkey: str = "f6", on_toggle: Optional[Callable[[], None]] = None) -> None:
        self._hotkey_str = hotkey.lower()
        self._on_toggle = on_toggle
        self._listener: Optional[keyboard.Listener] = None
        self._recording = False
        self._on_recorded: Optional[Callable[[str], None]] = None
        
        # Set to track keys currently held down (useful for combos later, though currently we focus on single keys)
        self._pressed_keys: Set[str] = set()
        
    def start(self) -> None:
        """Starts the global keyboard listener."""
        self.stop()
        
        # keyboard.Listener runs its own daemon thread
        self._listener = keyboard.Listener(
            on_press=self._on_press,
            on_release=self._on_release
        )
        self._listener.start()

    def stop(self) -> None:
        """Stops the global keyboard listener."""
        if self._listener is not None:
            self._listener.stop()
            self._listener = None

    def _key_to_string(self, key: Any) -> str:
        """
        Converts a pynput key to our string format.
        Handles both Special keys (keyboard.Key.f6 -> 'f6') and 
        Alphanumeric keys (keyboard.KeyCode(char='a') -> 'a').
        """
        if isinstance(key, keyboard.Key):
            # Key.f6 -> "f6", Key.ctrl_l -> "ctrl_l"
            return key.name.lower()
        elif hasattr(key, 'char') and key.char is not None:
            return key.char.lower()
        else:
            return str(key).lower()

    def _on_press(self, key: Any) -> None:
        """Called when a key is pressed."""
        key_str = self._key_to_string(key)
        self._pressed_keys.add(key_str)
        
        if self._recording:
            # We are in 'Change Hotkey' mode. Capture the key and stop recording.
            self._recording = False
            if self._on_recorded:
                # We could potentially build combo strings here using _pressed_keys,
                # but for simplicity we'll just capture the primary key pressed.
                self._on_recorded(key_str)
        else:
            # Normal operation: check if hotkey matches
            if self._matches_hotkey(key_str):
                if self._on_toggle:
                    self._on_toggle()

    def _on_release(self, key: Any) -> None:
        """Called when a key is released."""
        key_str = self._key_to_string(key)
        if key_str in self._pressed_keys:
            self._pressed_keys.remove(key_str)

    def _matches_hotkey(self, key_str: str) -> bool:
        """Check if the pressed key matches the configured hotkey."""
        # Simple match for now. In a full implementation, you'd check if
        # all modifiers in _hotkey_str (e.g. 'ctrl+f6') are present in _pressed_keys.
        return key_str == self._hotkey_str

    def set_hotkey(self, hotkey_str: str) -> None:
        """Updates the active hotkey."""
        self._hotkey_str = hotkey_str.lower()

    def start_recording(self, callback: Callable[[str], None]) -> None:
        """
        Enters recording mode. The very next key pressed will be captured
        and passed to the callback instead of triggering normal toggle behavior.
        """
        self._on_recorded = callback
        self._recording = True

    def stop_recording(self) -> None:
        """Exits recording mode without capturing a key."""
        self._recording = False
        self._on_recorded = None

    @property
    def hotkey(self) -> str:
        """Get the current hotkey string."""
        return self._hotkey_str
