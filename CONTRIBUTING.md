# Contributing to Klick

Thank you for considering contributing to Klick!

## Development Setup

1. **Fork and Clone**:
   ```bash
   git clone https://github.com/efeartn/Klick.git
   cd Klick
   ```
2. **Install Rust** via [rustup.rs](https://rustup.rs/) (stable toolchain).
3. **Linux X11 / OpenGL Build Dependencies** (Ubuntu/Debian):
   ```bash
   sudo apt-get install -y libgl1-mesa-dev libegl1-mesa-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb-shape0-dev libxcb-xfixes0-dev libwayland-dev
   ```
4. **Build and Test**:
   ```bash
   cargo test
   cargo run --release
   ```

## Code Style

- Format changes with `cargo fmt` and verify zero warnings with `cargo clippy --all-targets`.
- Keep OS-specific hooks and input injection isolated inside `#[cfg(target_os = ...)]` modules.

## Pull Request Process

1. Create a feature branch (`git checkout -b feature/your-change`).
2. Add or update unit tests in the relevant `src/core/` module.
3. Run `cargo test` and verify `--smoke-test` passes cleanly.
4. Commit with clear, descriptive messages and open a Pull Request against `main`.

## Issue Reporting

When reporting a bug, please include:
- OS and Klick version (`Klick --version`).
- Reproduction steps, expected behavior, and actual behavior.
