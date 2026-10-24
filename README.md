<div align="center">
  <img src="assets/logo.svg" alt="ClickForge Logo" width="112" height="112">

  # ClickForge

  **High-precision desktop auto-clicker with native Win32 input injection, hybrid sub-millisecond timing, and Gaussian humanization.**

  [![License: MIT](https://img.shields.io/badge/License-MIT-7C3AED.svg)](LICENSE)
  [![Platform: Windows | macOS | Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-25252B.svg)](#installation)
  [![Python: 3.9+](https://img.shields.io/badge/Python-3.9%2B-3776AB.svg)](https://www.python.org/downloads/)
</div>

---

ClickForge is a lightweight, open-source desktop auto-clicker engineered for deterministic timing accuracy, low CPU overhead, and a clean, distraction-free interface. On Windows, it bypasses high-level GUI automation wrappers by talking directly to `user32.SendInput` with pre-allocated C structures and a hybrid sleep + spin-wait timer loop.

## Technical Capabilities

- **Native Input Injection (`SendInput`):** Pre-allocated Win32 `INPUT` structures dispatch mouse events directly through `ctypes` on Windows (with a portable `pynput` fallback on macOS and Linux), eliminating per-click object allocation overhead.
- **Hybrid Sub-Millisecond Timer:** Combines `winmm.timeBeginPeriod(1)` 1 ms scheduler resolution with coarse interruptible sleeps (`threading.Event.wait`) and a final `time.perf_counter()` spin-wait window (~1.5 ms) for sub-millisecond accuracy up to 1,000 CPS.
- **Guaranteed Mouse-Up Failsafe:** The worker loop tracks held button state inside a strict `try...finally` guard so stopping via hotkey, fixed count completion, panic key, or application exit always releases any depressed mouse button.
- **Gaussian Humanizer (Anti-Pattern Randomization):**
  - **Interval Jitter (`± % σ`):** Applies clamped Gaussian (normal) noise around the base interval instead of flat uniform randomness.
  - **Variable Hold Duty Cycle (`min–max ms`):** Randomizes the press-to-release duration between `mouseDown` and `mouseUp` using a triangular distribution.
  - **Position Micro-Jitter (`± px`):** Distributes fixed-coordinate clicks uniformly within a configurable pixel radius.
- **Flexible Triggering (`Toggle` & `Hold`) + Side Mouse Buttons:** Bind any keyboard key or mouse side button (`Mouse4` / `XButton1`, `Mouse5` / `XButton2`) in either Toggle or Hold-to-Click mode.
- **Per-Monitor DPI Awareness & Interactive Coordinate Picker:** Enables Windows Per-Monitor DPI Awareness (`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`) prior to window creation and provides a full-desktop crosshair overlay with live `X, Y` coordinates across multi-monitor setups.
- **Interval & Target CPS Modes:** Configure speed either by time interval (`ms`, `sec`, `min`) or directly by target `CPS`, with live conversion and atomic profile presets (`Default`, `Fast`, `Humanized`, `Custom`).

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

# Build standalone single-file executable (dist/ClickForge.exe)
python build.py --onefile
```

### One-Line PyInstaller Command

To package a standalone, console-free `.exe` directly with PyInstaller:

```bash
python -m PyInstaller --noconfirm --clean --noconsole --onefile --name ClickForge --icon=assets/icon.ico --add-data "assets;assets" --collect-all customtkinter main.py
```

### Regenerating Icons & Vector Assets

All brand assets (`assets/logo.svg`, `assets/icon.png`, and the multi-DPI `assets/icon.ico`) are generated deterministically via Pillow:

```bash
python scripts/generate_assets.py
```

---

## Project Structure

```text
ClickForge/
├── assets/
│   ├── icon.ico                  # Multi-DPI Windows icon (16, 24, 32, 48, 64, 128, 256 px)
│   ├── icon.png                  # 512x512 high-resolution PNG icon
│   └── logo.svg                  # Scalable vector logo
├── clickforge/
│   ├── core/
│   │   ├── clicker.py            # Worker loop, Gaussian humanizer, guaranteed mouse-up failsafe
│   │   ├── config.py             # Schema-validated JSON persistence and profile presets
│   │   ├── hotkeys.py            # Global keyboard + Mouse4/Mouse5 trigger & panic listener
│   │   ├── input_backend.py      # Pre-allocated ctypes Win32 SendInput + pynput fallback
│   │   ├── system.py             # Per-Monitor v2 DPI awareness, AppUserModelID, screen bounds
│   │   ├── timing.py             # winmm 1ms timer resolution + hybrid sleep/spin-wait
│   │   └── validation.py         # Numeric sanitizers and interval/CPS unit converters
│   ├── ui/
│   │   ├── main_window.py        # Main CustomTkinter window & 18 Hz telemetry poller
│   │   ├── picker.py             # Fullscreen crosshair coordinate picker overlay
│   │   ├── theme.py              # Light/dark surface tokens and font resolution
│   │   └── widgets.py            # Reusable cards, segmented controls, and numeric entries
│   ├── __init__.py               # Package version and metadata
│   ├── __main__.py               # python -m clickforge entry point
│   └── app.py                    # Application launcher
├── scripts/
│   ├── build.bat                 # Windows one-command build wrapper
│   └── generate_assets.py        # Deterministic SVG/PNG/ICO asset generator
├── build.py                      # Cross-platform PyInstaller build script
├── clickforge.spec               # PyInstaller specification
├── main.py                       # Root entry script
├── pyproject.toml                # PEP 621 package configuration
└── requirements.txt              # Runtime dependencies
```

---

## License

Released under the [MIT License](LICENSE).
