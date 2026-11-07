#!/usr/bin/env python3
"""Single-command build helper for ClickForge.

Usage:
    python build.py            # Folder distribution (instant startup)
    python build.py --zip      # Folder distribution + release .zip archive
    python build.py --onefile  # Standalone single-file executable
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import zipfile

ROOT = os.path.dirname(os.path.abspath(__file__))


def _zip_directory_contents(src_dir: str, zip_path: str, keep_top_dir: bool = False) -> None:
    """Compress ``src_dir`` into ``zip_path`` matching CI release layout."""
    base = os.path.dirname(src_dir) if keep_top_dir else src_dir
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for dirpath, _dirnames, filenames in os.walk(src_dir):
            for fname in sorted(filenames):
                full = os.path.join(dirpath, fname)
                zf.write(full, os.path.relpath(full, base))


def _package_release_zip() -> str:
    if sys.platform.startswith("win"):
        src = os.path.join("dist", "ClickForge")
        out = os.path.join("dist", "ClickForge-Windows.zip")
        _zip_directory_contents(src, out, keep_top_dir=False)
    elif sys.platform == "darwin":
        src = os.path.join("dist", "ClickForge.app")
        out = os.path.join("dist", "ClickForge-macOS.zip")
        _zip_directory_contents(src, out, keep_top_dir=True)
    else:
        src = os.path.join("dist", "ClickForge")
        out = os.path.join("dist", "ClickForge-Linux.zip")
        _zip_directory_contents(src, out, keep_top_dir=False)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Build ClickForge distributable.")
    parser.add_argument(
        "--onefile",
        action="store_true",
        help="Build a single-file standalone executable instead of a directory bundle.",
    )
    parser.add_argument(
        "--zip",
        action="store_true",
        help="Also package the folder bundle into a release .zip archive under dist/.",
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

    rc = subprocess.call(cmd)
    if rc == 0 and args.zip and not args.onefile:
        archive = _package_release_zip()
        print(f"Packaged release archive: {archive}")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
