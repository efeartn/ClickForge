<div align="center">

<img src="assets/icon.png" alt="ClickForge logo" width="84" />

# ClickForge

**High-precision, modern desktop auto-clicker for Windows, macOS, and Linux.**

Built with CustomTkinter, Win32 `SendInput`, 1 ms multimedia timing (`timeBeginPeriod`), and an Apple-inspired Obsidian & Porcelain palette.

[![Latest Release](https://img.shields.io/github/v/release/efeartn/ClickForge?color=7C3AED&label=Release)](https://github.com/efeartn/ClickForge/releases/latest)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-52525B)](#download--install)

[![Download Windows Setup](https://img.shields.io/badge/Windows%20Installer-ClickForge--Setup.exe-7C3AED?style=for-the-badge&logo=windows)](https://github.com/efeartn/ClickForge/releases/latest/download/ClickForge-Setup.exe)
&nbsp;
[![Portable & Other Platforms](https://img.shields.io/badge/All%20Downloads-GitHub%20Releases-27272A?style=for-the-badge)](https://github.com/efeartn/ClickForge/releases/latest)

</div>

---

## Screenshots

<table>
  <tr>
    <td align="center" width="50%">
      <strong>Unified Workspace — Dark Mode</strong><br/><br/>
      <img src="docs/screenshots/app-dark.png" alt="ClickForge Dark Mode" width="380" />
    </td>
    <td align="center" width="50%">
      <strong>Unified Workspace — Light Mode</strong><br/><br/>
      <img src="docs/screenshots/app-light.png" alt="ClickForge Light Mode" width="380" />
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <strong>Windows Programs Setup</strong><br/><br/>
      <img src="docs/screenshots/setup-prompt.png" alt="ClickForge Windows Setup Dialog" width="340" />
    </td>
    <td align="center" width="50%">
      <strong>Automatic Update Prompt</strong><br/><br/>
      <img src="docs/screenshots/update-prompt.png" alt="ClickForge Update Prompt" width="340" />
    </td>
  </tr>
</table>

---

## Ready Out of the Box

ClickForge opens directly into one clean, complete workspace — no mode switcher, no setup friction:

- **Follow Cursor by Default:** Clicks land wherever your pointer is immediately on first launch (switch to **Fixed XY** + multi-monitor **Pick** anytime).
- **`10 ms` (`~100 CPS`) Default Speed:** Snappy out of the box, scalable from 1 click/hour up to **`1,000 CPS`** (`1 ms` interval).
- **Working `F6` Trigger & `Esc` Panic:** Press **`F6`** immediately on launch to start/stop, toggle or hold, bind **Mouse4 / Mouse5** or Numpad keys, or hit **`Esc`** / fling pointer to `(0, 0)` for instant emergency stop.
- **Fast First Launch:** Folder-based distribution (`_internal/`), withdrawn single-pass layout, and cached icons launch cleanly without single-file extraction delays.

---

## Download & Install

| Platform | Package | Notes |
| --- | --- | --- |
| **Windows (Installer)** | [`ClickForge-Setup.exe`](https://github.com/efeartn/ClickForge/releases/latest/download/ClickForge-Setup.exe) | **Recommended.** Adds ClickForge to Start Menu & Windows Installed Apps (`%LOCALAPPDATA%\Programs\ClickForge`) with no admin/UAC prompt and automatic self-updates. |
| **Windows (Portable)** | [`ClickForge-Windows.zip`](https://github.com/efeartn/ClickForge/releases/latest/download/ClickForge-Windows.zip) | Extract and run `ClickForge.exe`. Includes a **Setup** button in the top bar to install into Windows Programs anytime. |
| **macOS** | [`ClickForge-macOS.zip`](https://github.com/efeartn/ClickForge/releases/latest/download/ClickForge-macOS.zip) | Unzip `ClickForge.app`. Grant **Accessibility** access under *System Settings → Privacy & Security* so global hotkeys and mouse injection work. |
| **Linux (X11)** | [`ClickForge-Linux.zip`](https://github.com/efeartn/ClickForge/releases/latest/download/ClickForge-Linux.zip) | Unzip and run `./ClickForge`. Requires an X11 session for global keyboard/mouse hooks. |

---

## Features

### Precision & High-CPS Engine
- **Windows `SendInput` Fast Path:** Pre-allocated ctypes `INPUT` structures avoid per-click allocation overhead, sustaining up to **`1,000 CPS`**.
- **Sub-Millisecond Scheduler (`timing.py`):** Acquires `timeBeginPeriod(1)` only while clicking is active and pairs interruptible `Event.wait` with a light final spin (`1.5 ms` threshold).
- **Thread-Safe Guaranteed Release:** Stopping mid-hold or closing during a run always emits `mouseUp` so buttons never get stuck.
- **Live Telemetry:** Real-time rolling 1-second **Live CPS** readout and total click counter.

### Controls, Presets & Humanizer
- **Flexible Units & Repeats:** Enter speeds in **`ms`**, **`sec`**, **`min`**, or **`CPS`**; run **Infinite** or **Fixed** click counts.
- **Buttons & Click Types:** **Left**, **Right**, or **Middle** mouse buttons with **Single (`1x`)** or **Double (`2x`)** click cadence.
- **Presets (`Default`, `Fast`, `Humanized`, `Custom`):** One-click speed profiles (`Fast` = `500 CPS`) plus JSON profile import/export.
- **Humanizer:** Optional Gaussian interval jitter (`± % σ`), triangular randomized button hold duty (`ms`), and uniform disk coordinate spread (`± px`).

### Desktop Ergonomics & Auto-Updater
- **Windows Setup Integration:** Official `ClickForge-Setup.exe` wizard plus an in-app **Setup** button when running uninstalled on Windows.
- **Automatic Startup Update Check:** Checks GitHub Releases on launch, previews `"What's New"` release notes, verifies byte size + **`SHA-256`** checksum, and stages rollback-safe updates.
- **Header Quick Controls:** Toggle **Sound** start/stop chimes, cycle **System / Dark / Light** theme, toggle **Always-on-Top**, or check updates manually.

---

## Run from Source

Requires **Python 3.11+**.

```bash
git clone https://github.com/efeartn/ClickForge.git
cd ClickForge
pip install -r requirements.txt
python main.py
```

Run tests:

```bash
python -m unittest discover -v tests
```

Build release bundle (`ClickForge-Windows.zip` and `ClickForge-Setup.exe` with Inno Setup 6):

```bash
python build.py --zip --installer
```

---

## Architecture

```text
ClickForge/
├── clickforge/
│   ├── __init__.py             # App name, version, and release codename
│   ├── app.py                  # Window launch entrypoint
│   ├── core/
│   │   ├── clicker.py          # Worker thread engine, Humanizer, corner failsafe
│   │   ├── config.py           # Schema-validated persistent config & preset JSON
│   │   ├── hotkeys.py          # Global keyboard & Mouse4/5 listener with VK normalization
│   │   ├── input_backend.py    # Win32 SendInput backend + pynput fallback
│   │   ├── installer.py        # Windows Programs & Start Menu installer helpers
│   │   ├── sound.py            # Non-blocking start/stop chimes
│   │   ├── system.py           # DPI awareness, multi-monitor bounds, asset paths
│   │   ├── timing.py           # HighResolutionTimer + precision wait_until
│   │   ├── updater.py          # GitHub Releases updater with SHA-256 & rollback
│   │   └── validation.py       # Numeric parsing and speed/unit conversions
│   └── ui/
│       ├── main_window.py      # Primary application window & event poll loop
│       ├── picker.py           # Multi-monitor fixed XY coordinate picker overlay
│       ├── theme.py            # Light/Dark Obsidian & Porcelain palette tokens
│       └── widgets.py          # Reusable cards, segmented pills, modal dialogs
├── installer/
│   └── ClickForge.iss          # Inno Setup 6 script for ClickForge-Setup.exe
├── scripts/
│   └── generate_assets.py      # Vector-rendered app icons (.ico, .icns, .png, .svg)
└── tests/
    └── test_clickforge.py      # Unit & UI test suite
```

---

## License

Licensed under the [GNU General Public License v3.0](LICENSE).
