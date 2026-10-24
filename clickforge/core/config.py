"""Persistent settings and profile presets.

Settings live in a JSON file in the platform's user config directory.  Every
value is validated against :data:`DEFAULTS` on load, so a corrupted or
hand-edited file can never crash the app.
"""

from __future__ import annotations

import copy
import json
import os
import tempfile
from typing import Any, Dict

from platformdirs import user_config_dir

SCHEMA_VERSION = 3

#: Fields that make up a profile (what presets control).
PROFILE_DEFAULTS: Dict[str, Any] = {
    "speed_value": 100.0,
    "speed_unit": "ms",          # ms | sec | min | cps
    "repeat_mode": "infinite",   # infinite | fixed
    "repeat_count": 10,
    "button": "left",            # left | right | middle
    "click_type": "single",      # single | double
    "position_mode": "cursor",   # cursor | fixed
    "fixed_x": 0,
    "fixed_y": 0,
    "humanize": False,
    "jitter_pct": 15.0,
    "hold_min_ms": 18.0,
    "hold_max_ms": 42.0,
    "pos_jitter_px": 2,
}

#: App-wide settings (not affected by presets).
DEFAULTS: Dict[str, Any] = {
    "schema": SCHEMA_VERSION,
    "profile": "Default",
    "theme": "System",           # System | Light | Dark
    "always_on_top": False,
    "hotkey": "f6",
    "trigger_mode": "toggle",    # toggle | hold
    "settings": dict(PROFILE_DEFAULTS),
    "custom": dict(PROFILE_DEFAULTS),
}

_CHOICES = {
    "speed_unit": ("ms", "sec", "min", "cps"),
    "repeat_mode": ("infinite", "fixed"),
    "button": ("left", "right", "middle"),
    "click_type": ("single", "double"),
    "position_mode": ("cursor", "fixed"),
    "theme": ("System", "Light", "Dark"),
    "trigger_mode": ("toggle", "hold"),
}

PROFILE_NAMES = ("Default", "Fast", "Humanized", "Custom")

# Presets intentionally leave position fields alone so switching profiles
# never discards a carefully picked coordinate.
PRESETS: Dict[str, Dict[str, Any]] = {
    "Default": {
        "speed_value": 100.0, "speed_unit": "ms", "repeat_mode": "infinite",
        "button": "left", "click_type": "single", "humanize": False,
    },
    "Fast": {
        "speed_value": 100.0, "speed_unit": "cps", "repeat_mode": "infinite",
        "button": "left", "click_type": "single", "humanize": False,
    },
    "Humanized": {
        "speed_value": 12.0, "speed_unit": "cps", "repeat_mode": "infinite",
        "button": "left", "click_type": "single", "humanize": True,
        "jitter_pct": 15.0, "hold_min_ms": 18.0, "hold_max_ms": 42.0, "pos_jitter_px": 2,
    },
}


def _coerce(key: str, value: Any, default: Any) -> Any:
    """Return ``value`` converted to the type of ``default``, or ``default``."""
    if key in _CHOICES:
        return value if value in _CHOICES[key] else default
    try:
        if isinstance(default, bool):
            return value if isinstance(value, bool) else default
        if isinstance(default, int):
            return int(value)
        if isinstance(default, float):
            v = float(value)
            return v if v == v and v not in (float("inf"), float("-inf")) else default
        if isinstance(default, str):
            return value if isinstance(value, str) and value else default
    except (TypeError, ValueError):
        return default
    return default


def _validate_profile(data: Any) -> Dict[str, Any]:
    src = data if isinstance(data, dict) else {}
    return {k: _coerce(k, src.get(k, d), d) for k, d in PROFILE_DEFAULTS.items()}


class Config:
    def __init__(self, path: str | None = None) -> None:
        # Same location as v2.x so existing settings are migrated in place.
        directory = user_config_dir("ClickForge", "ClickForge")
        self.path = path or os.path.join(directory, "config.json")
        self._data: Dict[str, Any] = copy.deepcopy(DEFAULTS)
        self.load()

    # ── Access ───────────────────────────────────────────────────────

    def get(self, key: str) -> Any:
        return copy.deepcopy(self._data.get(key, DEFAULTS.get(key)))

    def set(self, key: str, value: Any) -> None:
        if key in ("settings", "custom"):
            value = _validate_profile(value)
        elif key in DEFAULTS:
            value = _coerce(key, value, DEFAULTS[key])
        self._data[key] = value

    def preset(self, name: str) -> Dict[str, Any]:
        """Full profile dict for ``name`` layered on the current settings."""
        current = self.get("settings")
        if name == "Custom":
            return self.get("custom")
        merged = dict(current)
        merged.update(PRESETS.get(name, {}))
        return _validate_profile(merged)

    # ── Persistence ──────────────────────────────────────────────────

    def load(self) -> None:
        try:
            with open(self.path, "r", encoding="utf-8") as f:
                raw = json.load(f)
        except (OSError, ValueError):
            return
        if not isinstance(raw, dict):
            return
        if raw.get("schema") != SCHEMA_VERSION:
            raw = self._migrate(raw)
        for key, default in DEFAULTS.items():
            if key in ("settings", "custom"):
                self._data[key] = _validate_profile(raw.get(key))
            elif key in raw:
                self._data[key] = _coerce(key, raw[key], default)
        self._data["schema"] = SCHEMA_VERSION

    def save(self) -> None:
        directory = os.path.dirname(self.path)
        try:
            os.makedirs(directory, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=directory, prefix=".config-", suffix=".tmp")
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(self._data, f, indent=2)
            os.replace(tmp, self.path)  # atomic: never leaves a half-written file
        except OSError:
            pass

    @staticmethod
    def _migrate(old: Dict[str, Any]) -> Dict[str, Any]:
        """Map the flat v2.x config onto the current schema."""
        settings = dict(PROFILE_DEFAULTS)
        unit = old.get("interval_unit", "ms")
        settings.update({
            "speed_value": old.get("interval", 100.0),
            "speed_unit": unit if unit in ("ms", "sec") else "ms",
            "repeat_mode": "fixed" if old.get("click_mode") == "fixed_count" else "infinite",
            "repeat_count": old.get("click_count", 10),
            "button": old.get("mouse_button", "left"),
            "click_type": old.get("click_type", "single"),
            "position_mode": old.get("position_mode", "cursor"),
            "fixed_x": old.get("fixed_x", 0),
            "fixed_y": old.get("fixed_y", 0),
            "humanize": bool(old.get("humanize", False)),
        })
        return {
            "hotkey": old.get("hotkey", "f6"),
            "theme": old.get("appearance_mode", "System"),
            "settings": settings,
            "custom": settings,
            "profile": "Custom",
        }
