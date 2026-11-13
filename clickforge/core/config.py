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
from typing import Any, Dict, Optional

from platformdirs import user_config_dir

from clickforge.core.hotkeys import normalize_binding

SCHEMA_VERSION = 5

#: Fields that make up a profile (what presets control).
PROFILE_DEFAULTS: Dict[str, Any] = {
    "speed_value": 10.0,
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
    "sound_cue": False,          # subtle audible cue on start/stop
    "hotkey": "f6",
    "trigger_mode": "toggle",    # toggle | hold
    "auto_update_check": True,
    "last_update_check": 0.0,    # unix time of the last automatic check
    "skipped_version": "",       # release the user chose to skip, e.g. "2.3.0"
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

_MAY_BE_EMPTY = frozenset({"skipped_version"})

# Presets keep saved coordinates intact unless returning to Default (which
# resets Follow Cursor so the clicker clicks where the user points).
PRESETS: Dict[str, Dict[str, Any]] = {
    "Default": {
        "speed_value": 10.0, "speed_unit": "ms", "repeat_mode": "infinite",
        "button": "left", "click_type": "single", "position_mode": "cursor",
        "humanize": False,
    },
    "Fast": {
        "speed_value": 500.0, "speed_unit": "cps", "repeat_mode": "infinite",
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
            if not isinstance(value, str):
                return default
            if key == "hotkey":
                return normalize_binding(value, default)
            return value if (value or key in _MAY_BE_EMPTY) else default
    except (TypeError, ValueError):
        return default
    return default


def _validate_profile(data: Any) -> Dict[str, Any]:
    src = data if isinstance(data, dict) else {}
    out = {k: _coerce(k, src.get(k, d), d) for k, d in PROFILE_DEFAULTS.items()}
    if out["speed_value"] <= 0:
        out["speed_value"] = PROFILE_DEFAULTS["speed_value"]
    out["repeat_count"] = max(1, min(int(out["repeat_count"]), 10_000_000))
    out["fixed_x"] = max(-32768, min(int(out["fixed_x"]), 32767))
    out["fixed_y"] = max(-32768, min(int(out["fixed_y"]), 32767))
    out["jitter_pct"] = max(0.0, min(float(out["jitter_pct"]), 50.0))
    h_min = max(0.0, min(float(out["hold_min_ms"]), 500.0))
    h_max = max(0.0, min(float(out["hold_max_ms"]), 500.0))
    if h_min > h_max:
        h_min, h_max = h_max, h_min
    out["hold_min_ms"] = h_min
    out["hold_max_ms"] = h_max
    out["pos_jitter_px"] = max(0, min(int(out["pos_jitter_px"]), 50))
    return out


def normalize_profile(data: Any) -> Dict[str, Any]:
    """Validated, comparison-friendly copy of a profile dict."""
    out = _validate_profile(data)
    for key, value in out.items():
        if isinstance(value, float):
            out[key] = round(value, 4)
    return out


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

    # ── Preset Import / Export ───────────────────────────────────────

    def export_profile(self, dest_path: str, profile: Optional[Dict[str, Any]] = None) -> None:
        """Write a validated preset profile to ``dest_path`` as JSON."""
        clean = normalize_profile(profile if profile is not None else self.get("settings"))
        payload = {
            "app": "ClickForge",
            "schema": SCHEMA_VERSION,
            "profile": clean,
        }
        with open(dest_path, "w", encoding="utf-8") as f:
            json.dump(payload, f, indent=2)

    def import_profile(self, src_path: str) -> Dict[str, Any]:
        """Read, validate, and apply a preset profile from ``src_path``."""
        with open(src_path, "r", encoding="utf-8") as f:
            raw = json.load(f)
        if not isinstance(raw, dict):
            raise ValueError("Preset file must contain a JSON object.")
        profile_src = raw.get("profile") if isinstance(raw.get("profile"), dict) else raw
        clean = normalize_profile(profile_src)
        self.set("profile", "Custom")
        self.set("settings", clean)
        self.set("custom", clean)
        self.save()
        return clean

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
        tmp: str | None = None
        try:
            os.makedirs(directory, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=directory, prefix=".config-", suffix=".tmp")
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(self._data, f, indent=2)
            os.replace(tmp, self.path)  # atomic: never leaves a half-written file
            tmp = None
        except OSError:
            if tmp is not None:
                try:
                    os.remove(tmp)
                except OSError:
                    pass

    @staticmethod
    def _migrate(old: Dict[str, Any]) -> Dict[str, Any]:
        """Migrate older schemas while preserving structured v2.2+ settings."""
        # Schema 3-4 already use structured 'settings' and 'custom' dicts.
        if isinstance(old.get("settings"), dict):
            migrated = dict(old)
            migrated.pop("ui_mode", None)
            if "hotkey" in migrated:
                migrated["hotkey"] = normalize_binding(migrated["hotkey"])
            # Heal legacy (0, 0) Fixed XY traps left over from removed Simple Mode,
            # and refresh untouched v3.x factory "Default" speed (100 ms -> 10 ms).
            for section in ("settings", "custom"):
                blk = migrated.get(section)
                if not isinstance(blk, dict):
                    continue
                patched = dict(blk)
                if (
                    patched.get("position_mode") == "fixed"
                    and int(patched.get("fixed_x", 0) or 0) == 0
                    and int(patched.get("fixed_y", 0) or 0) == 0
                ):
                    patched["position_mode"] = "cursor"
                if (
                    section == "settings"
                    and migrated.get("profile", "Default") == "Default"
                    and patched.get("speed_value") == 100.0
                    and patched.get("speed_unit", "ms") == "ms"
                ):
                    patched["speed_value"] = 10.0
                migrated[section] = patched
            return migrated

        # Legacy flat v2.0/v2.1 config
        settings = dict(PROFILE_DEFAULTS)
        unit = old.get("interval_unit", "ms")
        settings.update({
            "speed_value": old.get("interval", 10.0),
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
            "hotkey": normalize_binding(old.get("hotkey", "f6")),
            "theme": old.get("appearance_mode", "System"),
            "settings": settings,
            "custom": settings,
            "profile": "Custom",
        }
