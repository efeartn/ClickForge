# ClickForge — Technical Handoff (`v2.5.0`)

## 1. Project Snapshot
- **Project:** **ClickForge** — High-precision, cross-platform desktop auto-clicker (`Python 3.9+`, `CustomTkinter`, native Win32 `SendInput`, `pynput`).
- **Current Version:** `2.5.0`.
- **License:** `GPL-3.0-or-later`.
- **Repository:** `https://github.com/efeartn/ClickForge.git` (default branch: `main`).
- **GitHub Topics:** `auto-clicker`, `autoclicker`, `customtkinter`, `desktop-automation`, `gui`, `linux`, `macos`, `mouse-clicker`, `open-source`, `pynput`, `python`, `sendinput`, `utility`, `win32api`, `windows`.
- **Git & Release Workflow:**
  - Authenticated via CLI `git` (`2.56.0.windows.4`) through **Git Credential Manager**.
  - Commit author email: `261309133+efeartn@users.noreply.github.com` (protects privacy & prevents spam).
  - Pushing a `v*` tag (`git tag vX.Y.Z && git push origin main vX.Y.Z`) triggers `.github/workflows/build.yml`, which executes unit tests (`python -m unittest discover tests`), packages builds for Windows, macOS, and Linux, and publishes the GitHub Release automatically.

---

## 2. Core Design Philosophy & Guardrails
1. **Dual Modes for Everyone (Simple vs. Pro):**
   - **Simple Mode:** Frictionless, minimal layout for casual/everyday users: quick CPS pills (`5 CPS`, `10 CPS`, `20 CPS`, `Custom`), `Left` / `Right` click, clean repeat options (`Until Stopped` / `Fixed Count`), and a compact content-fit window. Zero developer jargon.
   - **Pro Mode:** Instant single-click toggle to the full power-user suite: Gaussian delay jitter, hold duty cycle, pixel dispersion, Fixed XY coordinate picker, multi-unit intervals (`ms`, `sec`, `min`, `CPS`), and preset profiles (`Default`, `Fast`, `Humanized`, `Custom`).
2. **Apple-Inspired Minimalist Aesthetic:**
   - Obsidian-to-graphite squircle surfaces, subtle specular borders (`#262633` dark / `#E2E2E8` light), softer rounded corners (`RADIUS = 10`), royal violet accent (`#7C3AED`), and a sculpted geometric mark with ambient glow and concentric touch rings.
3. **Auditory Confirmation:**
   - Subtle, non-intrusive auditory cues on start (1320 Hz) and stop (880 Hz) executed on a daemon thread (`clickforge/core/sound.py`) so users get instant feedback in full-screen games, toggleable via the header sound button.
4. **Thread Safety & Sub-Millisecond Accuracy:**
   - Dedicated daemon click worker (`ClickEngine`) and global input hooks (`TriggerListener`) never touch Tk widgets directly.
   - Native Win32 `SendInput` with pre-allocated C structures, `winmm.timeBeginPeriod(1)` 1 ms scheduler resolution, and hybrid sleep/spin-wait timing up to `1,000 CPS`.
   - Guaranteed lock-guarded mouse-up release on stop, panic, failsafe, or exit.

---

## 3. Architecture & Components

### `clickforge/core/` — Backend & Logic
- **`clicker.py` (`ClickSettings`, `ClickEngine`):**
  - Threaded click loop with Gaussian humanizer (clamped normal interval jitter, triangular hold duration, disk spatial micro-jitter), guaranteed mouse-up release guard, and top-left `(0, 0)` failsafe.
- **`config.py` (`Config`, `PRESETS`, `SCHEMA_VERSION = 4`):**
  - Atomic JSON persistence in `platformdirs.user_config_dir("ClickForge", "ClickForge")`.
  - Supports `ui_mode` (`"simple"` / `"pro"`), `sound_cue` toggle, and profile export/import via `export_profile()` / `import_profile()`.
- **`sound.py` (`play_cue`):**
  - Dedicated background thread dispatcher for subtle start/stop audio cues (Windows `winsound.Beep`, macOS `afplay` system sound).
- **`input_backend.py` (`Win32Backend`, `PynputBackend`):**
  - Direct Win32 `user32.SendInput` injection with `INJECTED_MARKER = 0x434C4B46`, with cross-platform `pynput` fallback.
- **`hotkeys.py` (`TriggerListener`):**
  - Global hotkeys (keyboard, numpad `Num 0`–`Num 9`, `Mouse4`/`Mouse5`) in `Toggle` or `Hold` mode with panic `Esc` key.
- **`timing.py` (`HighResolutionTimer`, `wait_until`, `wait_for`):**
  - 1 ms timer period via `winmm.timeBeginPeriod(1)` + interruptible sleep and spin-wait window.
- **`updater.py` (`fetch_latest`, `download_asset`, `stage_update`, `launch_installer`):**
  - GitHub Releases update checker with HTTPS enforcement, size & SHA-256 digest validation, and Windows detached self-installer with `.cfbak` rollback.
- **`validation.py`:**
  - Input filtering, speed parsing, and safe bidirectional interval/CPS conversions.
- **`system.py`:**
  - Per-Monitor v2 DPI awareness, virtual desktop bounds, and OS warnings.

### `clickforge/ui/` — Interface
- **`theme.py` & `widgets.py`:**
  - Apple-inspired light/dark surface tokens, platform font resolver (`Segoe UI Variable`, `SF Pro`, `Inter`), `Card`, `FormRow`, `Segmented`, `NumberEntry`, `StatusBadge`, and `UpdateBanner`.
- **`main_window.py` (`ClickForgeWindow`):**
  - Dynamic single-window interface with Mode Switcher bar (`Simple Mode` vs `Pro Mode`), header bar (Sound, Theme, Pin, Update, Status), and content-fit geometry preservation (`_fit_to_content`).
- **`picker.py` (`CoordinatePicker`):**
  - Full-desktop crosshair coordinate overlay with live `X, Y` readout across multi-monitor setups.

### Tests & CI
- **`tests/test_clickforge.py`:** 20 automated unit tests verifying metadata, config, validation, engine lifecycles, hotkeys, audio cues, theme tokens, and updater logic.
- **`.github/workflows/build.yml`:** Automated CI running `python -m unittest discover tests` across Windows, macOS, and Linux runners prior to PyInstaller packaging.

---

## 4. Verification & Testing
- Run test suite:
  ```powershell
  python -m unittest discover tests
  ```
- Run app from source:
  ```powershell
  python main.py
  ```
- Regenerate brand assets:
  ```powershell
  python scripts/generate_assets.py
  ```
