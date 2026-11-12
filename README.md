<div align="center">
  <img src="assets/logo.svg" alt="ClickForge Logo" width="112" height="112">

  # ClickForge

  **High-precision desktop auto-clicker with native Win32 input injection, hybrid sub-millisecond timing, and Gaussian humanization.**

  [![Release](https://img.shields.io/github/v/release/efeartn/ClickForge?color=7C3AED&label=Release)](https://github.com/efeartn/ClickForge/releases/latest)
  [![Build Status](https://github.com/efeartn/ClickForge/actions/workflows/build.yml/badge.svg)](https://github.com/efeartn/ClickForge/actions/workflows/build.yml)
  [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-7C3AED.svg)](LICENSE)
  [![Platform: Windows | macOS | Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-25252B.svg)](#installation)
  [![Python: 3.9+](https://img.shields.io/badge/Python-3.9%2B-3776AB.svg)](https://www.python.org/downloads/)
</div>

---

ClickForge is a lightweight, open-source desktop auto-clicker engineered for deterministic timing accuracy, low CPU overhead, and a clean, distraction-free interface. On Windows, it bypasses high-level GUI automation wrappers by talking directly to `user32.SendInput` with pre-allocated C structures and a hybrid sleep + spin-wait timer loop.

## Technical Capabilities

- **Dual Modes (Simple & Pro):**
  - **Simple Mode:** Frictionless setup for everyday users — pick from quick CPS presets (`5 CPS`, `10 CPS`, `20 CPS`, `Custom`), select **Left** or **Right** click, choose **Until Stopped** or **Fixed Count**, and press **Start**.
  - **Pro Mode:** Complete power-user suite — custom interval units (`ms`, `sec`, `min`, `CPS`), **Follow Cursor** vs **Fixed XY** positioning with fullscreen crosshair coordinate picker, full **Gaussian Humanizer** controls, and preset profiles (`Default`, `Fast`, `Humanized`, `Custom`).
- **Native Input Injection (`SendInput`):** Pre-allocated Win32 `INPUT` structures dispatch mouse events directly through `ctypes` on Windows (with a portable `pynput` fallback on macOS and Linux), eliminating per-click object allocation overhead.
- **Hybrid Sub-Millisecond Timer:** Combines `winmm.timeBeginPeriod(1)` 1 ms scheduler resolution with coarse interruptible sleeps (`threading.Event.wait`) and a final `time.perf_counter()` spin-wait window (~1.5 ms) for sub-millisecond accuracy up to 1,000 CPS.
- **Guaranteed Mouse-Up Failsafe:** The worker loop tracks held button state inside a strict `try...finally` guard so stopping via hotkey, fixed count completion, panic key, or application exit always releases any depressed mouse button.
- **Gaussian Humanizer (Anti-Pattern Randomization):**
  - **Interval Jitter (`± % σ`):** Applies clamped Gaussian (normal) noise around the base interval instead of flat uniform randomness.
  - **Variable Hold Duty Cycle (`min–max ms`):** Randomizes the press-to-release duration between `mouseDown` and `mouseUp` using a triangular distribution.
  - **Position Micro-Jitter (`± px`):** Distributes fixed-coordinate clicks uniformly within a configurable pixel radius.
- **Subtle Audio Cues (Sound Cue):** Plays an instant, low-latency auditory chime when clicking starts (1320 Hz) and stops (880 Hz) in the background so you have clear confirmation even while running full-screen games.
- **Profile Preset Export & Import:** Export and import custom configuration profiles as JSON to back up and share tailored auto-clicking setups.
- **Flexible Triggering (`Toggle` & `Hold`) + Side Mouse Buttons:** Bind any keyboard key or mouse side button (`Mouse4` / `XButton1`, `Mouse5` / `XButton2`) in either Toggle or Hold-to-Click mode.
- **Per-Monitor DPI Awareness & Interactive Coordinate Picker:** Enables Windows Per-Monitor DPI Awareness (`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`) prior to window creation, dynamically sizes the window to prevent DPI clipping, and provides a full-desktop crosshair overlay with live `X, Y` coordinates across multi-monitor setups.
- **Verified In-App Auto-Updater:** Checks GitHub Releases in the background (or on demand from the header bar), verifies archive size and `SHA-256` digests before staging, and performs a safe in-place self-update with automatic `.cfbak` rollback on Windows packaged builds.

---

## Installation

### Pre-Built Binaries

Download the archive for your platform from the [Releases](https://github.com/efeartn/ClickForge/releases) page:

- **Windows:** Extract `ClickForge-Windows.zip` and run `ClickForge.exe`.
- **macOS:** Extract `ClickForge-macOS.zip` and open `ClickForge.app` (grant Accessibility permissions when prompted).
- **Linux:** Extract `ClickForge-Linux.zip` and run `./ClickForge` under an X11 or XWayland session.

### Run from Source

```bash
git clone https://github.com/efeartn/ClickForge.git
cd ClickForge
pip install -r requirements.txt
python main.py
```

---

## Usage

1. **Speed & Repeat:** Choose `ms`, `sec`, `min`, or `CPS` and enter your desired value. Select **Infinite** to click until stopped, or **Fixed** to stop after a specific number of clicks.
2. **Action & Target:** Select **Left**, **Right**, or **Middle** mouse button and **1x** (single) or **2x** (double) click type. Choose **Follow Cursor** or **Fixed XY** (click **Pick** to select a screen coordinate with the crosshair overlay).
3. **Humanizer:** Enable **Humanizer** to configure Gaussian delay jitter (`± % σ`), spatial spread (`± px`), and hold duration (`min` to `max` ms).
4. **Trigger & Panic Stop:**
   - Choose **Toggle** (press once to start, again to stop) or **Hold** (click only while holding the trigger).
   - Click the **Hotkey** button to bind any keyboard key or `Mouse4` / `Mouse5`.
   - **Emergency Stop:** Press `Esc` at any time or move the pointer to the top-left corner `(0, 0)` of the primary display to unconditionally halt clicking.

---

## Build from Source

### Using the Build Helper

```bash
# Build instant-launch folder bundle (dist/ClickForge/)
python build.py

# Build folder bundle + release archive (dist/ClickForge-Windows.zip)
python build.py --zip

# Build standalone single-file executable (dist/ClickForge.exe)
python build.py --onefile
```

### One-Line PyInstaller Command

To package a standalone, console-free `.exe` directly with PyInstaller:

```bash
python -m PyInstaller --noconfirm --clean --noconsole --onefile --name ClickForge --icon=assets/icon.ico --add-data "assets;assets" --collect-all customtkinter main.py
```

### Regenerating Icons & Vector Assets

All brand assets (`assets/logo.svg`, `assets/icon.png`, `assets/icon.ico`, and `assets/icon.icns`) are generated deterministically via Pillow:

```bash
python scripts/generate_assets.py
```

---

## Project Structure

```text
ClickForge/
├── assets/
│   ├── icon.icns                 # macOS app bundle icon (1024x1024 master)
│   ├── icon.ico                  # Multi-DPI Windows icon (16, 24, 32, 48, 64, 128, 256 px)
│   ├── icon.png                  # 512x512 high-resolution PNG icon
│   └── logo.svg                  # Scalable vector logo
├── clickforge/
│   ├── core/
│   │   ├── clicker.py            # Worker loop, Gaussian humanizer, guaranteed mouse-up failsafe
│   │   ├── config.py             # Schema-validated JSON persistence, presets, and profile export/import
│   │   ├── hotkeys.py            # Global keyboard + Mouse4/Mouse5 trigger & panic listener
│   │   ├── input_backend.py      # Pre-allocated ctypes Win32 SendInput + pynput fallback
│   │   ├── sound.py              # Background auditory state cues (start/stop chimes)
│   │   ├── system.py             # Per-Monitor v2 DPI awareness, AppUserModelID, screen bounds
│   │   ├── timing.py             # winmm 1ms timer resolution + hybrid sleep/spin-wait
│   │   ├── updater.py            # GitHub Releases updater, SHA-256 verifier & self-installer
│   │   └── validation.py         # Numeric sanitizers and interval/CPS unit converters
│   ├── ui/
│   │   ├── main_window.py        # Main CustomTkinter window, Simple & Pro modes, 50 Hz event loop
│   │   ├── picker.py             # Fullscreen crosshair coordinate picker overlay
│   │   ├── theme.py              # Apple-inspired light/dark surface tokens and font resolution
│   │   └── widgets.py            # Reusable cards, segmented controls, and numeric entries
│   ├── __init__.py               # Package version and metadata
│   ├── __main__.py               # python -m clickforge entry point
│   └── app.py                    # Application launcher
├── scripts/
│   ├── build.bat                 # Windows one-command build wrapper
│   └── generate_assets.py        # Deterministic SVG/PNG/ICO/ICNS asset generator
├── tests/
│   └── test_clickforge.py        # Automated test suite (20 unit tests)
├── build.py                      # Cross-platform PyInstaller build script
├── clickforge.spec               # PyInstaller specification
├── main.py                       # Root entry script
├── pyproject.toml                # PEP 621 package configuration
└── requirements.txt              # Runtime dependencies
```

---

## License

Released under the [GNU General Public License v3.0](LICENSE).
