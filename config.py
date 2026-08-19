import json
import os
from pathlib import Path
from platformdirs import user_config_dir

class Config:
    """
    Manages persistent user settings via JSON file.
    Uses platformdirs to find the appropriate configuration directory for the OS.
    """
    APP_NAME = "ClickForge"
    CONFIG_FILENAME = "config.json"
    MIN_INTERVAL_MS = 10
    MAX_INTERVAL_MS = 3_600_000
    MAX_CLICK_COUNT = 1_000_000

    DEFAULT_SETTINGS = {
        "interval": 100,           # Click interval value
        "interval_unit": "ms",     # "ms" or "seconds"
        "mouse_button": "left",    # "left", "right", "middle"
        "click_type": "single",    # "single" or "double"
        "click_mode": "continuous", # "continuous" or "fixed_count"
        "click_count": 10,         # Target clicks for fixed_count mode
        "position_mode": "cursor", # "cursor" or "fixed"
        "fixed_x": 0,              # Fixed click X coordinate
        "fixed_y": 0,              # Fixed click Y coordinate
        "hotkey": "f6",            # Global toggle hotkey
        "appearance_mode": "dark", # "dark", "light", or "system"
    }

    def __init__(self) -> None:
        """Initialize the Config instance and load settings."""
        self._config_path = self._get_config_path()
        self._settings = self.DEFAULT_SETTINGS.copy()
        self.load()

    def _get_config_path(self) -> Path:
        """
        Returns the full path to the config.json file.
        Using platformdirs ensures we follow OS standards:
        - Windows: %APPDATA%\\Local\\ClickForge\\config.json or similar
        - macOS: ~/Library/Application Support/ClickForge/config.json
        - Linux: ~/.config/ClickForge/config.json
        """
        config_dir = Path(user_config_dir(self.APP_NAME))
        return config_dir / self.CONFIG_FILENAME

    def load(self) -> None:
        """
        Load settings from the JSON file. 
        Merges loaded settings with default settings to handle missing keys.
        Handles corrupt files gracefully by falling back to defaults.
        """
        if not self._config_path.exists():
            return
            
        try:
            with open(self._config_path, 'r', encoding='utf-8') as f:
                loaded_settings = json.load(f)
                
            # Merge with defaults to ensure all keys exist
            for key in self.DEFAULT_SETTINGS:
                if key in loaded_settings:
                    self._settings[key] = loaded_settings[key]
        except (json.JSONDecodeError, OSError) as e:
            # If file is corrupt or unreadable, stick with defaults
            print(f"Error loading config: {e}. Falling back to defaults.")

    def save(self) -> None:
        """
        Write current settings to the JSON file.
        Creates parent directories if they don't exist.
        """
        try:
            # Ensure the directory exists
            self._config_path.parent.mkdir(parents=True, exist_ok=True)
            
            with open(self._config_path, 'w', encoding='utf-8') as f:
                json.dump(self._settings, f, indent=4)
        except OSError as e:
            print(f"Error saving config: {e}")

    def get(self, key: str, default: any = None) -> any:
        """Get a setting value by key."""
        return self._settings.get(key, default)

    def set(self, key: str, value: any) -> None:
        """Set a setting value with basic validation."""
        if key not in self.DEFAULT_SETTINGS:
            return
            
        # Basic validation
        if key == "interval":
            # Convert to int, but don't strictly enforce bounds here 
            # as unit might change. get_interval_ms will enforce bounds.
            try:
                value = int(value)
            except ValueError:
                return
        elif key == "mouse_button" and value not in ["left", "right", "middle"]:
            return
        elif key == "click_type" and value not in ["single", "double"]:
            return
            
        self._settings[key] = value

    def get_interval_ms(self) -> int:
        """
        Returns the interval converted to milliseconds, 
        ensuring it falls within allowed bounds.
        """
        interval = self._settings.get("interval", 100)
        unit = self._settings.get("interval_unit", "ms")
        
        ms = interval
        if unit == "seconds":
            ms = interval * 1000
            
        # Clamp to bounds to prevent abuse or lockups
        return max(self.MIN_INTERVAL_MS, min(self.MAX_INTERVAL_MS, ms))

    def reset(self) -> None:
        """Reset all settings to default and save."""
        self._settings = self.DEFAULT_SETTINGS.copy()
        self.save()

    def to_dict(self) -> dict:
        """Return a copy of all current settings."""
        return self._settings.copy()
