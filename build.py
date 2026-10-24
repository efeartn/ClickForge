#!/usr/bin/env python3
"""Single-command build helper for ClickForge.

Usage:
    python build.py            # Folder distribution (instant startup)
    python build.py --onefile  # Standalone single-file executable
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))


def main() -> int:
    parser = argparse.ArgumentParser(description="Build ClickForge distributable.")
    parser.add_argument(
        "--onefile",
        action="store_true",
        help="Build a single-file standalone executable instead of a directory bundle.",
    )
    args = parser.parse_args()

    os.chdir(ROOT)

    # 1. Ensure assets exist and are up to date
    subprocess.check_call([sys.executable, os.path.join("scripts", "generate_assets.py")])

    # 2. Clean previous build artifacts
    for folder in ("build", "dist"):
        if os.path.exists(folder):
            shutil.rmtree(folder)

    # 3. Run PyInstaller
    if args.onefile:
        sep = ";" if sys.platform.startswith("win") else ":"
        cmd = [
            sys.executable,
            "-m",
            "PyInstaller",
            "--noconfirm",
            "--clean",
            "--noconsole",
            "--onefile",
            "--name=ClickForge",
            "--icon=assets/icon.ico",
            f"--add-data=assets{sep}assets",
            "--collect-all=customtkinter",
            "main.py",
        ]
    else:
        cmd = [
            sys.executable,
            "-m",
            "PyInstaller",
            "clickforge.spec",
            "--clean",
            "-y",
        ]

    return subprocess.call(cmd)


if __name__ == "__main__":
    raise SystemExit(main())
