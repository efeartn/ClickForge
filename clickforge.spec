# -*- mode: python ; coding: utf-8 -*-
import os
import sys
import customtkinter

sys.path.insert(0, os.path.abspath("."))
from clickforge import __version__

block_cipher = None
ctk_path = os.path.dirname(customtkinter.__file__)

hidden_imports = ["customtkinter", "PIL", "PIL._tkinter_finder"]
if sys.platform.startswith("win"):
    hidden_imports.extend(["pynput.keyboard._win32", "pynput.mouse._win32"])
elif sys.platform == "darwin":
    hidden_imports.extend(["pynput.keyboard._darwin", "pynput.mouse._darwin"])
else:
    hidden_imports.extend(["pynput.keyboard._xorg", "pynput.mouse._xorg"])

datas = [
    (ctk_path, "customtkinter"),
    ("assets/icon.ico", "assets"),
    ("assets/icon.png", "assets"),
    ("assets/logo.svg", "assets"),
]
if os.path.exists("assets/icon.icns"):
    datas.append(("assets/icon.icns", "assets"))

if not sys.platform.startswith("win"):
    try:
        from PyInstaller.utils.hooks import collect_data_files
        import certifi  # noqa: F401

        datas.extend(collect_data_files("certifi"))
        hidden_imports.append("certifi")
    except Exception:
        pass

a = Analysis(
    ["main.py"],
    pathex=[],
    binaries=[],
    datas=datas,
    hiddenimports=hidden_imports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        "unittest",
        "test",
        "doctest",
        "pydoc",
        "lib2to3",
        "ensurepip",
        "idlelib",
        "tkinter.test",
        "pip",
        "setuptools",
        "wheel",
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="ClickForge" if sys.platform != "darwin" else "ClickForge_Mac",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    icon="assets/icon.ico" if sys.platform != "darwin" else None,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="ClickForge",
)

if sys.platform == "darwin":
    app = BUNDLE(
        coll,
        name="ClickForge.app",
        icon="assets/icon.icns" if os.path.exists("assets/icon.icns") else None,
        bundle_identifier="com.efeartn.clickforge",
        info_plist={
            "CFBundleShortVersionString": __version__,
            "CFBundleVersion": __version__,
            "NSHighResolutionCapable": "True",
            "LSBackgroundOnly": "False",
        },
    )
