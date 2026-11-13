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

from clickforge import __version__

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


def _find_iscc() -> str | None:
    for candidate in (
        shutil.which("iscc"),
        shutil.which("ISCC.exe"),
        r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        r"C:\Program Files\Inno Setup 6\ISCC.exe",
    ):
        if candidate and os.path.isfile(candidate):
            return candidate
    return None


def _build_windows_installer() -> str:
    iscc = _find_iscc()
    if iscc is None:
        raise FileNotFoundError(
            "Inno Setup 6 (ISCC.exe) not found. Install it from https://jrsoftware.org/isinfo.php"
        )
    iss_path = os.path.join("installer", "ClickForge.iss")
    subprocess.check_call([iscc, f"/DAppVersion={__version__}", iss_path])
    return os.path.join("dist", "ClickForge-Setup.exe")


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
    parser.add_argument(
        "--installer",
        action="store_true",
        help="On Windows, compile ClickForge-Setup.exe using Inno Setup 6.",
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
    if rc == 0 and args.installer and not args.onefile:
        if not sys.platform.startswith("win"):
            print("--installer is only supported on Windows.", file=sys.stderr)
            return 2
        setup_exe = _build_windows_installer()
        print(f"Built Windows installer: {setup_exe}")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
