# Changelog

All notable changes to **ClickForge** are documented in this file.

## [3.3.1] - 2026-11-19 — "Carbon"

### Fixed
- **macOS & Linux Hotkey KeyCode Collisions**: Resolved non-special `KeyCode` bindings via character before platform virtual-key numbers on macOS and Linux X11 in [hotkeys.py](clickforge/core/hotkeys.py), preventing keys like `-` (macOS `vk=27`) or `u` (X11 `vk=117`) from misfiring as Windows `Esc` or `F6`.
- **macOS & Linux Release Bundle Packaging**: Updated `.github/workflows/build.yml` to package macOS `.app` bundles with `ditto --sequesterRsrc` and Linux bundles with `zip -ry` + `chmod +x`, preserving framework and shared library symlinks, and added CI frozen-binary `--smoke-test` validation on Windows, macOS, and Linux.
- **macOS & Linux Picker, Screen Bounds, Audio & Backend**: Added CoreGraphics multi-monitor bounds on macOS, enabled desktop grabs and parent `<Escape>` + global panic cancellation in `CoordinatePicker`, extended macOS `afplay` timeout and added `canberra-gtk-play`/`paplay` cue playback on Linux, and overrode `PynputBackend.click()` to emit direct single clicks.
- **Follow-Cursor Jitter Drift & Tooltip Auto-Hide**: Anchored follow-cursor Humanizer spread (`position_jitter_px`) to prevent Brownian cursor drift when stationary (re-anchoring smoothly when the user moves the pointer), and auto-hid tooltips after 6s or when losing window focus/mapping.
- **Cross-Platform Header Polishing**: Guarded Windows-only `.ico` timers so macOS and Linux avoid unnecessary delayed callbacks.

### Changed & Optimized
- **Cross-Platform 2x Vector Header Icons**: Added [icons.py](clickforge/ui/icons.py) to render cached 4x-supersampled `14x14` (`@2x`) PIL/CTkImage icons for Sound, Theme, Pin, and Update across Windows, macOS, and Linux instead of falling back to wide text buttons off Windows.
- **Live Speed Preview & Lower Idle Overhead**: Updated effective CPS / interval preview live while typing (`<KeyRelease>`), relaxed event polling from `20 ms` to `35 ms` when idle, and added lightweight `--version` and `--smoke-test` CLI switches.

## [3.3.0] - 2026-11-16 — "Carbon"

### Changed
- **Carbon & Precision Emerald UI Overhaul**: Replaced the violet dark/light palette with neutral carbon/graphite dark surfaces (`#0A0B0D` / `#121418`), clean alabaster light mode surfaces, and restrained Precision Emerald accents (`#10A37F` / `#10B981`). Regenerated matching `.png`, `.ico`, `.icns`, and `.svg` icons plus the full-desktop coordinate picker overlay, styled the global hotkey selector as a tactile keycap button, changed idle status pill copy to **Ready**, and added a 1px divider between telemetry and presets.
- **Adaptive & Compact Layout**: Dynamically collapses `Fixed XY` coordinates while in `Follow Cursor` mode, collapses `Humanizer` controls when disabled (showing an `Off` / `Active` state pill), and hides the fixed repeat count input in `Infinite` mode (`~614` logical px default height with High-DPI safe screen centering).

### Added
- **Live Run Telemetry**: Added fixed-repeat progress formatting (`Total: X / Y`), an active/last-run `Elapsed: MM:SS.d` timer, and zero-allocation shared hover tooltips across header icons and version metadata.

### Fixed
- **Instant Stop CPS Reset**: Reset the `LIVE CPS` display immediately to `0` when clicking stops instead of letting rolling timestamps decay for up to a second.
- **Follow-Cursor Humanizer Spread**: Fixed `Spread ± px` (`position_jitter_px`) so humanizer cursor spread now works cleanly in both `Follow Cursor` and `Fixed XY` modes.
- **Idle CPU & RAM Optimizations**: Skipped redundant UI telemetry polling while idle/settled, freed the full 512x512 PIL logo buffer immediately after creating a 68x68 thumbnail, lazy-loaded modal dialog artwork, removed redundant multi-size `PhotoImage` allocations on Windows, and trimmed unused stdlib/Pillow modules from the PyInstaller bundle (`clickforge.spec`).

## [3.2.1] - 2026-11-14 — "Velocity"

### Fixed
- **Windows / CustomTkinter Invisible Launch Fix**: Removed premature `self.withdraw()` / `self.deiconify()` calls from `ClickForgeWindow.__init__()` so `CTk.mainloop()` does not leave the main window permanently hidden after setting the Windows title bar (`ClickForge.exe` previously ran in the background with no visible window).
- **Regression Coverage**: Added a `mainloop()` visibility test verifying the main window reaches `state() == "normal"` and maps cleanly on startup.

## [3.2.0] - 2026-11-13 — "Velocity"

### Changed
- **Unified Single Interface (Pro Mode Only):** Removed the Simple vs. Pro mode switcher so every user immediately gets the complete, uncluttered ClickForge workspace (Live CPS telemetry, multi-unit speed control, Target & Fixed XY picker, Humanizer, Trigger mode, and sound toggle) without extra clicks.
- **Snappier Out-of-the-Box Defaults:** Changed the factory default interval to **`10 ms` (`~100 CPS`)** and explicitly enforced **`Follow Cursor`** on the `Default` preset (`Fast` preset raised to `500 CPS`).
- **Faster First-Launch Paint & Slimmer Bundle:** Withdrew window construction until layout settles, cached resized window/logo bitmaps instead of decoding icons multiple times during startup, and excluded dev-only modules from the PyInstaller spec while keeping full `1,000 CPS` timer precision.

### Added
- **Windows Setup Installer (`ClickForge-Setup.exe`) & In-App `Setup` Button:** Added an Inno Setup 6 script (`installer/ClickForge.iss`) and CI packaging so ClickForge can be installed cleanly per-user into `%LOCALAPPDATA%\Programs\ClickForge` (registered in Start Menu and Windows Installed Apps / Programs without UAC prompts, with automatic updates still enabled). Portable Windows builds also include a header **Setup** button (`clickforge/core/installer.py`) to install directly into Programs or launch `ClickForge-Setup.exe`.

### Fixed
- **`F6` / `VK117` & `Esc` Hotkey Bug on Windows:** Fixed `TriggerListener._key_name()` calling `pynput`'s `canonical(key)` before checking `isinstance(key, keyboard.Key)`, which stripped `Key.f6` -> `KeyCode(vk=117)` and `Key.esc` -> `KeyCode(vk=27)`. Added `normalize_binding()` and a virtual-key lookup table so `F6` and `Esc` work immediately out of the box, recording `F6` saves `"f6"` cleanly, and legacy `"vk117"` configs auto-heal to `"f6"`.
- **Legacy Config Safety Migration (`Schema 5`):** Automatically upgrades `"vk117"` bindings, heals accidental `(0, 0)` Fixed XY traps from removed Simple Mode back to `Follow Cursor`, and refreshes untouched `Default` speeds to `10 ms`.

---

## [3.1.0] - 2026-11-13 — "Beacon"

### Added
- **Every-Launch Automatic Update Check:** Removed the 6-hour startup suppression window so ClickForge automatically checks GitHub Releases shortly after opening (`600 ms`) whenever `auto_update_check` is enabled.
- **Interactive Update Prompt Dialog (`UpdatePromptDialog`):** Added a themed modal prompt centered on the app window (including multi-monitor desktops) displaying version details, formatted `"What's New"` release notes, and clear choices (**Update Now** / **Not Now** / **Skip Version**).
- **Non-Intrusive Active-Clicking Fallback:** If the click engine is already active when an update check finishes, ClickForge displays the inline `UpdateBanner` instead of popping a dialog over active clicking; starting via hotkey also dismisses any open prompt cleanly.

### Fixed
- **Clean Shutdown Timer Cancellation:** Tracked and canceled scheduled startup, icon, and polling `after` timers in `_on_close()` and `UpdatePromptDialog.close()` to prevent late Tkinter callbacks during rapid exit.

---

## [3.0.0] - 2026-11-12 — "Obsidian"

### Added
- **Dual UI Modes (Simple vs. Pro):** Integrated an Apple-style segmented mode switcher. **Simple Mode** presents a minimal, frictionless interface for casual users with quick CPS pills (`5 CPS`, `10 CPS`, `20 CPS`, `Custom`), Left/Right button choice, clean repeat controls, and dynamic content-fit window resizing. **Pro Mode** expands full controls for power users (Gaussian jitter, hold duty cycle, coordinate picker, multi-unit intervals, and preset profiles).
- **In-App Release Codename Display:** Promoted the major milestone to **v3.0.0 ("Obsidian")** and embedded the release name directly in the application header (`v3.0.0 · Obsidian`).
- **Apple-Inspired Minimalist Brand Mark & Design Language:** Crafted a new minimalist geometric logo featuring a deep obsidian-to-graphite squircle, ambient violet glow, concentric precision touch rings, and a sculpted chevron pointer. Upgraded UI surfaces with Apple-style neutral tokens, refined specular borders, and softer corner radii (`RADIUS = 10`).
- **Auditory Feedback Cues:** Added subtle, non-blocking auditory chimes when clicking starts (1320 Hz) and stops (880 Hz) so users get instant feedback in fullscreen apps, toggleable via a new header icon button (`Sound` / `Mute`).
- **Profile Preset Export & Import:** Added JSON profile export and import capabilities in `Config` to back up, share, and migrate auto-clicking profiles.
- **Tracked Unit Test Suite & CI Integration:** Added a comprehensive test suite in `tests/test_clickforge.py` (20 unit tests) covering configuration, migration, input parsing, timing conversions, engine lifecycles, hotkeys, and updater logic, with automatic test verification wired into `.github/workflows/build.yml`.

### Fixed
- **Window Positioning on Dynamic Layout Shifts:** Fixed `_fit_to_content` when resizing between Simple and Pro modes so the window preserves its exact on-screen position instead of jumping.
- **Update Banner Attachment Safety:** Fixed update banner layout anchoring so notifications attach safely above content even when Pro telemetry cards are unpacked.

---

## [2.4.0] - 2026-11-10 — "Sentinel"

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

## [2.3.0] - 2026-11-07 — "Vanguard"

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

## [2.2.0] - 2026-08-20 — "Precision"

### Added
- **Native Win32 `SendInput` Backend:** Pre-allocated `ctypes` input structures on Windows with portable `pynput` fallback on macOS and Linux.
- **Hybrid Sub-Millisecond Timer:** `winmm.timeBeginPeriod(1)` 1 ms scheduler resolution paired with interruptible sleeps and a final spin-wait window.
- **Gaussian Humanizer:** Clamped Gaussian interval jitter (`± % σ`), triangular hold duration (`min–max ms`), and fixed-position pixel spread (`± px`).
- **Fluent / Linear UI Overhaul:** High-contrast dark/light theme tokens, segmented controls, live rolling 1-second CPS telemetry, and fullscreen crosshair coordinate picker.
- **Mouse Side Button Triggers:** Support for `Mouse4` (`XButton1`) and `Mouse5` (`XButton2`) in both `Toggle` and `Hold` modes.

---

## [2.1.0] - 2026-09-20 — "Refine"

### Fixed
- Fixed coordinate input validation for negative screen positions.
- Fixed double-click metric tracking cadence and counter consistency.
- Fixed macOS bundle icon compatibility and added write permissions to release pipelines.

---

## [2.0.0] - 2026-08-20 — "Genesis"

### Added
- Complete rewrite from legacy Tkinter to modern CustomTkinter with Bento Grid layout.
- Added Gaussian Humanize Mode, multi-platform CI build pipeline, and custom theme auto-detection.
- Hotkey configuration and multi-button click support (Left, Right, Middle).
- Interval speed units (ms, sec, min, CPS) and repeat counters.

---

## [1.0.0] - 2026-08-19 — "Origin"

### Added
- Initial public release of ClickForge desktop auto-clicker.
- Core threading architecture, background click loop, and global hotkey listener.
