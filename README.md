<div align="center">
  <img src="assets/icon.png" alt="ClickForge Logo" width="128">
  
  # ClickForge
  
  **A modern, minimalist, ridiculously fast cross-platform auto-clicker built with Python.**

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
  [![Platform: Windows (macOS/Linux planned)](https://img.shields.io/badge/platform-Windows%20Only%20(For%20Now)-blue)](#)
  [![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
</div>

---

ClickForge is an open-source auto-clicker designed to be powerful enough for power users, but simple enough for anyone to use. Built as a vibe-coded learning resource, it solves common bottlenecks in Python desktop apps to achieve **200+ Clicks Per Second (CPS)** while maintaining a clean, modern GUI.

> **Note:** ClickForge is currently fully optimized and working for **Windows**. Support for macOS and Linux is on the roadmap and will be added in future updates!

## ✨ Features

- 🚀 **Extreme Performance:** Bypasses standard library limitations on Windows using direct `ctypes` (`SendInput`) and microsecond spin-waiting to achieve true 1ms (and sub-millisecond) delays.
- 🎨 **Minimalist UI:** A beautiful, responsive interface powered by `CustomTkinter` featuring seamless Dark/Light modes.
- ⌨️ **Smart Hotkeys:** Global start/stop hotkey (default `F6`) with easy, bug-free rebinding.
- 🎯 **Advanced Positioning:** Click at your active cursor, or pick a fixed coordinate on your screen.
- ⏱️ **Precision Intervals:** Supports floating-point intervals (e.g., `0.5 ms`) for when you need to push the absolute limits.
- 🔄 **Modes & Types:** Left, Right, or Middle mouse buttons. Single or Double clicks. Continuous mode or Fixed target counts.
- 💾 **Persistent Settings:** Remembers all your configurations between launches.

## 🚀 Download & Run

### For Regular Users (Windows)
1. Go to the **[Releases](../../releases)** tab on GitHub.
2. Download the latest `ClickForge.exe`.
3. Run it! No installation required.

*(macOS and Linux binaries coming soon!)*

---

## 🛠️ For Developers (Run from Source)

ClickForge is built to be a great learning resource for Python GUI and automation development.

### Prerequisites
- Python 3.8 or higher
- Git

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/ClickForge.git
   cd ClickForge
   ```

2. Install the required dependencies:
   ```bash
   pip install -r requirements.txt
   ```

3. Run the app:
   ```bash
   python main.py
   ```

### Building the Executable
To package the application into a standalone `.exe` using PyInstaller:
```bash
python -m PyInstaller clickforge.spec --clean
```
The compiled executable will be located in the `dist/` folder.

## 🧠 How it achieves 200+ CPS (The Secret Sauce)

Most Python auto-clickers cap out at around 60 CPS on Windows. This happens for two reasons:
1. **The `pynput` bottleneck:** Creating Python objects and traversing the ctypes wrapper for every single click is too slow.
2. **The Windows Timer Resolution:** Standard `time.sleep()` and `threading.Event.wait()` on Windows have a base resolution of ~15.6ms. If you ask Python to sleep for 1ms, it actually sleeps for 15.6ms.

**ClickForge solves this by:**
- Using a custom `_precise_wait` function that utilizes `time.perf_counter()` for a high-precision CPU spin-wait on micro-intervals, bypassing the OS tick limitation.
- Calling the Windows `SendInput` API directly via `ctypes` using pre-allocated C-structures, stripping out all wrapper overhead.

## 🤝 Contributing

Contributions are welcome! Whether it's porting the high-speed `SendInput` logic to macOS (`CGEvent`), adding new features, or fixing bugs:

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📝 License

Distributed under the MIT License. See `LICENSE` for more information.
