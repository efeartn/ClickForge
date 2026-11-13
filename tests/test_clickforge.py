"""Unit test suite for ClickForge.

Covers:
- Package version & metadata
- Config schema, migration, presets, import/export
- Input validation, speed parsing, unit conversions
- ClickSettings sanitization and ClickEngine lifecycle
- Hotkey parsing and conflict detection
- Audio cues (start/stop)
- Theme color tokens and metrics
- Auto-updater version comparison
"""

from __future__ import annotations

import os
import re
import shutil
import tempfile
import time
import unittest
from unittest.mock import patch

import clickforge
from clickforge.core import updater
from clickforge.core.clicker import ClickEngine, ClickSettings
from clickforge.core.config import (
    DEFAULTS,
    PROFILE_DEFAULTS,
    PROFILE_NAMES,
    SCHEMA_VERSION,
    Config,
    normalize_profile,
)
from clickforge.core.hotkeys import (
    conflicts_with_numeric_entry,
    display_name,
    is_mouse_binding,
)
from clickforge.core.input_backend import InputBackend, create_backend
from clickforge.core.sound import play_cue
from clickforge.core.validation import (
    allow_float,
    allow_int,
    allow_uint,
    clamp,
    fmt_number,
    from_interval_ms,
    parse_float,
    parse_int,
    sanitize_speed,
    to_interval_ms,
)
from clickforge.ui import theme as t
from clickforge.ui.main_window import ClickForgeWindow
from clickforge.ui.widgets import format_release_notes


class DummyBackend(InputBackend):
    """Headless dummy input backend for testing."""

    def __init__(self) -> None:
        self.pressed = []
        self.released = []
        self.clicks = []
        self.pos = (100, 100)

    def press(self, button: str) -> None:
        self.pressed.append(button)

    def release(self, button: str) -> None:
        self.released.append(button)

    def click(self, button: str) -> None:
        self.clicks.append(button)

    def move(self, x: int, y: int) -> None:
        self.pos = (x, y)

    def position(self):
        return self.pos


class TestMetadata(unittest.TestCase):
    def test_version_format(self):
        self.assertRegex(clickforge.__version__, r"^\d+\.\d+\.\d+$")
        self.assertEqual(clickforge.__version__, "3.1.0")
        self.assertEqual(clickforge.__codename__, "Beacon")
        self.assertEqual(clickforge.__app_name__, "ClickForge")


class TestConfig(unittest.TestCase):
    def setUp(self):
        self.tmp_dir = tempfile.mkdtemp()
        self.config_path = os.path.join(self.tmp_dir, "test_config.json")

    def tearDown(self):
        try:
            if os.path.exists(self.config_path):
                os.remove(self.config_path)
            os.rmdir(self.tmp_dir)
        except OSError:
            pass

    def test_schema_and_defaults(self):
        cfg = Config(path=self.config_path)
        self.assertEqual(cfg.get("schema"), SCHEMA_VERSION)
        self.assertIn(cfg.get("ui_mode"), ("simple", "pro"))
        self.assertEqual(cfg.get("ui_mode"), "simple")
        self.assertFalse(cfg.get("sound_cue"))
        self.assertEqual(cfg.get("hotkey"), "f6")
        self.assertEqual(cfg.get("trigger_mode"), "toggle")

    def test_save_and_load(self):
        cfg = Config(path=self.config_path)
        cfg.set("ui_mode", "pro")
        cfg.set("sound_cue", True)
        cfg.set("hotkey", "f8")
        cfg.save()

        cfg2 = Config(path=self.config_path)
        self.assertEqual(cfg2.get("ui_mode"), "pro")
        self.assertTrue(cfg2.get("sound_cue"))
        self.assertEqual(cfg2.get("hotkey"), "f8")

    def test_profile_export_import(self):
        cfg = Config(path=self.config_path)
        export_file = os.path.join(self.tmp_dir, "preset_export.json")
        try:
            sample_profile = {
                "speed_value": 25.0,
                "speed_unit": "cps",
                "repeat_mode": "fixed",
                "repeat_count": 500,
                "button": "right",
                "click_type": "single",
                "position_mode": "fixed",
                "fixed_x": 420,
                "fixed_y": 680,
                "humanize": True,
                "jitter_pct": 12.0,
                "hold_min_ms": 20.0,
                "hold_max_ms": 40.0,
                "pos_jitter_px": 4,
            }
            cfg.export_profile(export_file, sample_profile)
            self.assertTrue(os.path.isfile(export_file))

            imported = cfg.import_profile(export_file)
            self.assertEqual(imported["speed_value"], 25.0)
            self.assertEqual(imported["speed_unit"], "cps")
            self.assertEqual(imported["repeat_mode"], "fixed")
            self.assertEqual(imported["repeat_count"], 500)
            self.assertEqual(imported["button"], "right")
            self.assertEqual(imported["fixed_x"], 420)
            self.assertEqual(imported["fixed_y"], 680)
            self.assertTrue(imported["humanize"])
        finally:
            if os.path.exists(export_file):
                os.remove(export_file)

    def test_schema_migration(self):
        legacy = {
            "interval": 200.0,
            "interval_unit": "ms",
            "click_mode": "fixed_count",
            "click_count": 25,
            "mouse_button": "right",
            "hotkey": "f7",
            "appearance_mode": "Dark",
        }
        migrated = Config._migrate(legacy)
        self.assertEqual(migrated["ui_mode"], "simple")
        self.assertEqual(migrated["settings"]["speed_value"], 200.0)
        self.assertEqual(migrated["settings"]["repeat_mode"], "fixed")
        self.assertEqual(migrated["settings"]["repeat_count"], 25)
        self.assertEqual(migrated["settings"]["button"], "right")
        self.assertEqual(migrated["hotkey"], "f7")


class TestValidation(unittest.TestCase):
    def test_keystroke_filters(self):
        self.assertTrue(allow_float("12.5"))
        self.assertTrue(allow_float("100"))
        self.assertFalse(allow_float("abc"))
        self.assertFalse(allow_float("-12.5"))

        self.assertTrue(allow_uint("42"))
        self.assertFalse(allow_uint("-42"))
        self.assertFalse(allow_uint("42.5"))

        self.assertTrue(allow_int("-50"))
        self.assertTrue(allow_int("120"))
        self.assertFalse(allow_int("12.5"))

    def test_unit_conversions(self):
        # 100 ms = 10 CPS
        self.assertAlmostEqual(to_interval_ms(100.0, "ms"), 100.0)
        self.assertAlmostEqual(to_interval_ms(10.0, "cps"), 100.0)
        self.assertAlmostEqual(to_interval_ms(0.5, "sec"), 500.0)
        self.assertAlmostEqual(to_interval_ms(1.0, "min"), 60000.0)

        self.assertAlmostEqual(from_interval_ms(100.0, "cps"), 10.0)
        self.assertAlmostEqual(from_interval_ms(500.0, "sec"), 0.5)

    def test_sanitize_speed(self):
        self.assertEqual(sanitize_speed("20", "cps"), 20.0)
        self.assertEqual(sanitize_speed("50", "ms"), 50.0)
        self.assertIsNone(sanitize_speed("", "ms"))
        self.assertIsNone(sanitize_speed("0", "cps"))
        self.assertIsNone(sanitize_speed("-5", "ms"))
        self.assertIsNone(sanitize_speed("invalid", "ms"))

    def test_fmt_number(self):
        self.assertEqual(fmt_number(10.0), "10")
        self.assertEqual(fmt_number(12.5), "12.5")
        self.assertEqual(fmt_number(0.125), "0.125")


class TestClickSettingsAndEngine(unittest.TestCase):
    def test_click_settings_sanitized(self):
        # Out-of-range clamping and hold duty min > max swap
        s = ClickSettings(
            interval_ms=-10.0,
            repeat_count=-5,
            jitter_pct=99.0,
            hold_min_ms=60.0,
            hold_max_ms=20.0,
            position_jitter_px=999,
        ).sanitized()

        self.assertGreaterEqual(s.interval_ms, 1.0)
        self.assertEqual(s.repeat_count, 0)
        self.assertEqual(s.jitter_pct, 50.0)
        # Swapped so min <= max
        self.assertEqual(s.hold_min_ms, 20.0)
        self.assertEqual(s.hold_max_ms, 60.0)
        self.assertEqual(s.position_jitter_px, 50)

    def test_engine_lifecycle(self):
        backend = DummyBackend()
        stopped_events = []
        engine = ClickEngine(backend=backend, on_stopped=lambda r: stopped_events.append(r))

        self.assertFalse(engine.is_running)
        self.assertFalse(engine.is_active)

        settings = ClickSettings(interval_ms=10.0, repeat_count=5).sanitized()
        started = engine.start(settings)
        self.assertTrue(started)

        # Wait for 5 clicks to complete
        time.sleep(0.15)
        self.assertFalse(engine.is_active)
        self.assertGreaterEqual(engine.click_count, 5)
        self.assertIn("completed", stopped_events)

        engine.reset_counter()
        self.assertEqual(engine.click_count, 0)

    def test_guaranteed_mouse_up(self):
        backend = DummyBackend()
        engine = ClickEngine(backend=backend)
        engine._held = "left"
        engine.release_held()
        self.assertIn("left", backend.released)
        self.assertIsNone(engine._held)


class TestHotkeys(unittest.TestCase):
    def test_display_names(self):
        self.assertEqual(display_name("f6"), "F6")
        self.assertEqual(display_name("f11"), "F11")
        self.assertEqual(display_name("space"), "Space")
        self.assertEqual(display_name("mouse:x1"), "Mouse4")
        self.assertEqual(display_name("mouse:x2"), "Mouse5")
        self.assertEqual(display_name("vk96"), "Num 0")
        self.assertEqual(display_name("vk105"), "Num 9")
        self.assertEqual(display_name("vk107"), "Num +")

    def test_numeric_conflict(self):
        # Conflicting numeric characters and editing keys
        for key in ("0", "5", "9", ".", "-", "vk96", "backspace", "delete"):
            self.assertTrue(conflicts_with_numeric_entry(key), f"Key {key} should conflict")
        # Non-conflicting hotkeys
        for key in ("f6", "f8", "space", "mouse:x1", "mouse:x2", "q"):
            self.assertFalse(conflicts_with_numeric_entry(key), f"Key {key} should not conflict")

    def test_is_mouse_binding(self):
        self.assertTrue(is_mouse_binding("mouse:x1"))
        self.assertTrue(is_mouse_binding("mouse:x2"))
        self.assertFalse(is_mouse_binding("f6"))


class TestAudioCue(unittest.TestCase):
    def test_audio_cue_safe(self):
        # Disabled cue is a no-op
        play_cue("start", enabled=False)
        play_cue("stop", enabled=False)
        # Enabled cue dispatches to a background thread without error
        play_cue("start", enabled=True)
        play_cue("stop", enabled=True)


class TestThemeTokens(unittest.TestCase):
    def test_colors_are_hex_tuples(self):
        hex_pattern = re.compile(r"^#[0-9A-Fa-f]{6}$")
        for token_name in ("BG", "SURFACE", "FIELD", "BORDER", "ACCENT", "DANGER", "SUCCESS"):
            token = getattr(t, token_name)
            self.assertIsInstance(token, tuple, f"Token {token_name} must be a tuple")
            self.assertEqual(len(token), 2, f"Token {token_name} must have 2 elements (light, dark)")
            self.assertRegex(token[0], hex_pattern, f"Token {token_name}[0] must be hex color")
            self.assertRegex(token[1], hex_pattern, f"Token {token_name}[1] must be hex color")

    def test_metrics(self):
        self.assertGreater(t.RADIUS, 0)
        self.assertGreater(t.CONTROL_H, 0)
        self.assertGreater(t.PAD, 0)


class TestUpdater(unittest.TestCase):
    def test_version_parsing(self):
        self.assertEqual(updater.parse_version("v2.5.0"), (2, 5, 0))
        self.assertEqual(updater.parse_version("2.5.0"), (2, 5, 0))
        self.assertEqual(updater.parse_version("2.4"), (2, 4, 0))
        self.assertIsNone(updater.parse_version("not-a-version"))

    def test_is_newer(self):
        self.assertTrue(updater.is_newer((2, 6, 0), (2, 5, 0)))
        self.assertTrue(updater.is_newer((2, 5, 1), (2, 5, 0)))
        self.assertFalse(updater.is_newer((2, 5, 0), (2, 5, 0)))
        self.assertFalse(updater.is_newer((2, 4, 0), (2, 5, 0)))

    def test_format_release_notes(self):
        raw = "## ClickForge v3.1.0\n\n### Highlights\n- **Auto Check:** Works on launch.\n- Added `v3.1.0` support."
        cleaned = format_release_notes(raw)
        self.assertIn("ClickForge v3.1.0", cleaned)
        self.assertIn("\u2022 Auto Check: Works on launch.", cleaned)
        self.assertIn("\u2022 Added v3.1.0 support.", cleaned)
        self.assertNotIn("##", cleaned)
        self.assertNotIn("**", cleaned)
        self.assertNotIn("`", cleaned)


class TestStartupUpdatePrompt(unittest.TestCase):
    def setUp(self):
        self.tmp_dir = tempfile.mkdtemp()
        self.cfg = Config(path=os.path.join(self.tmp_dir, "config.json"))
        self.newer = updater.ReleaseInfo(
            tag="v9.9.0",
            version=(9, 9, 0),
            title='v9.9.0 — "Future"',
            page_url="https://github.com/efeartn/ClickForge/releases/tag/v9.9.0",
            notes="## Highlights\n- **Faster:** Improved startup update checks.",
            assets=(),
        )
        self.current = updater.ReleaseInfo(
            tag=f"v{clickforge.__version__}",
            version=updater.current_version(),
            title=f"v{clickforge.__version__}",
            page_url="https://github.com/efeartn/ClickForge/releases/latest",
            notes="",
            assets=(),
        )

    def tearDown(self):
        shutil.rmtree(self.tmp_dir, ignore_errors=True)

    def _make_window(self):
        with (
            patch("clickforge.core.hotkeys.TriggerListener.start"),
            patch("clickforge.ui.main_window.startup_warnings", return_value=[]),
        ):
            app = ClickForgeWindow(config=self.cfg)
            self.assertIsNotNone(app._startup_update_job)
            # Cancel scheduled background timers so unit tests control timing deterministically
            for attr in ("_startup_update_job", "_startup_warn_job"):
                job = getattr(app, attr, None)
                if job is not None:
                    app.after_cancel(job)
                    setattr(app, attr, None)
            app.update()
            return app

    def test_startup_check_runs_regardless_of_last_check_and_prompts_user(self):
        # Even if last_update_check was just set, opening the app still checks
        self.cfg.set("last_update_check", time.time())
        app = self._make_window()
        try:
            with patch.object(app, "_check_for_updates") as mock_check:
                app._maybe_auto_check_updates()
                mock_check.assert_called_once_with(manual=False)

            app._on_update_found((self.newer, False))
            app.update()
            self.assertIsNotNone(app._update_dialog)
            self.assertEqual(app._update_release, self.newer)

            # Clicking "Not Now" closes dialog without skipping the version
            app._update_dialog.later_btn.invoke()
            self.assertIsNone(app._update_dialog)
            self.assertEqual(self.cfg.get("skipped_version"), "")
        finally:
            app._on_close()

    def test_startup_check_respects_auto_update_disabled_and_skip(self):
        self.cfg.set("auto_update_check", False)
        app = self._make_window()
        try:
            with patch.object(app, "_check_for_updates") as mock_check:
                app._maybe_auto_check_updates()
                mock_check.assert_not_called()

            # Up-to-date release does not prompt on startup
            app._on_update_found((self.current, False))
            self.assertIsNone(app._update_dialog)

            # Newer release prompts, and Skip records skipped_version
            app._on_update_found((self.newer, False))
            app.update()
            self.assertIsNotNone(app._update_dialog)
            app._update_dialog._choose_skip()
            self.assertIsNone(app._update_dialog)
            self.assertEqual(self.cfg.get("skipped_version"), "9.9.0")

            # Subsequent startup auto-check does not re-open prompt for skipped version
            app._on_update_found((self.newer, False))
            self.assertIsNone(app._update_dialog)
        finally:
            app._on_close()

    def test_active_engine_uses_non_intrusive_banner_instead_of_dialog(self):
        app = self._make_window()
        try:
            with patch.object(
                type(app._engine), "is_active", property(lambda _self: True)
            ):
                app._on_update_found((self.newer, False))
                self.assertIsNone(app._update_dialog)
                self.assertTrue(bool(app.update_banner.winfo_manager()))
        finally:
            app._on_close()


if __name__ == "__main__":
    unittest.main()
