# Changelog

All notable changes to **ClickForge** are documented in this file.

## [2.4.0] - 2026-11-10

### Changed
- **License Migrated to GPL-3.0:** Relicensed ClickForge from MIT to the **GNU General Public License v3.0 (`GPL-3.0`)** to keep derivatives open-source.
- **Instant Hotkey Trigger While Editing:** Pressing a non-conflicting hotkey (such as `F6` or `Mouse4`/`Mouse5`) while a numeric field is focused now commits the edited value and starts clicking immediately instead of ignoring the trigger.
- **Numpad Virtual-Key Labels:** Numpad bindings (`vk96`–`vk111`) now render with clean labels (`Num 0`–`Num 9`, `Num +`, `Num -`, `Num *`, `Num /`, `Num .`).
- **Click-to-Cancel Hotkey Recording:** Clicking the hotkey button a second time while recording cancels the recording cleanly.

### Fixed
- **Corner Failsafe in Fixed XY Mode:** Moving the pointer to `(0, 0)` between clicks now halts Fixed XY runs as well (unless `(0, 0)` itself is the configured target).
- **Double-Click Cadence (`2x`):** Added a minimum inter-click gap for non-humanized double-clicks so target apps never drop or coalesce the second click.
- **Thread-Safe Button Release & Listener Shutdown:** Synchronized held-button release with an explicit lock so `stop(wait=True)` and worker exit never emit duplicate `mouseUp` events, and hardened `TriggerListener.stop()` / `reset_trigger_state()` against race conditions.
- **Config Profile Clamping & Temp Cleanup:** `_validate_profile` now clamps out-of-range values in hand-edited `config.json` files, and `Config.save()` removes orphaned `.tmp` files if `os.replace` fails.

---

## [2.3.0] - 2026-11-07

### Added
- **Verified In-App Auto-Updater (`clickforge/core/updater.py`):** Background and manual GitHub Releases update checker with HTTPS enforcement, size & `SHA-256` digest verification, ZipSlip extraction guards, and a detached Windows self-installer with automatic `.cfbak` rollback.
- **Inline Update Banner (`UpdateBanner`):** Non-blocking header notification bar with live download progress (`MB` / `%`), **Update**, **Later**, **Skip**, and **Restart & Update** controls.
- **macOS Bundle Icon (`assets/icon.icns`):** Deterministic `1024x1024` master icon generated via `scripts/generate_assets.py` and wired into `clickforge.spec`.
- **Release Archive Flag (`build.py --zip`):** Packages folder builds directly into platform release `.zip` archives.

### Fixed
- **DPI Window Clipping:** Replaced fixed `460x690` window geometry with dynamic content-fit sizing (`_fit_to_content`) so scaled displays never clip the footer.
- **Preset Selection Persistence:** Starting the clicker on `Default`, `Fast`, or `Humanized` no longer flips the active preset to `Custom` unless settings were modified.
- **Hold-Mode Release & Panic Stop Latency:** `Hold` trigger release and `Esc` panic stop now halt `ClickEngine` directly on the listener thread instead of waiting on the UI poll queue.
- **Rapid Re-Toggle Race:** `ClickEngine.start()` cleanly joins an unwinding worker thread so rapid stop/start hotkey taps are not dropped.

---

## [2.2.0] - 2026-08-20

### Added
- **Native Win32 `SendInput` Backend:** Pre-allocated `ctypes` input structures on Windows with portable `pynput` fallback on macOS and Linux.
- **Hybrid Sub-Millisecond Timer:** `winmm.timeBeginPeriod(1)` 1 ms scheduler resolution paired with interruptible sleeps and a final spin-wait window.
- **Gaussian Humanizer:** Clamped Gaussian interval jitter (`± % σ`), triangular hold duration (`min–max ms`), and fixed-position pixel spread (`± px`).
- **Fluent / Linear UI Overhaul:** High-contrast dark/light theme tokens, segmented controls, live rolling 1-second CPS telemetry, and fullscreen crosshair coordinate picker.
- **Mouse Side Button Triggers:** Support for `Mouse4` (`XButton1`) and `Mouse5` (`XButton2`) in both `Toggle` and `Hold` modes.
