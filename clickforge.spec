# -*- mode: python ; coding: utf-8 -*-
import os
import sys
import customtkinter

block_cipher = None
ctk_path = os.path.dirname(customtkinter.__file__)

hidden_imports = ['customtkinter']
if sys.platform.startswith('win'):
    hidden_imports.extend(['pynput.keyboard._win32', 'pynput.mouse._win32'])
elif sys.platform == 'darwin':
    hidden_imports.extend(['pynput.keyboard._darwin', 'pynput.mouse._darwin'])
else:
    hidden_imports.extend(['pynput.keyboard._xorg', 'pynput.mouse._xorg'])

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[
        (ctk_path, 'customtkinter'),
        ('assets/icon.ico', 'assets'),
        ('assets/fonts', 'assets/fonts'),
    ],
    hiddenimports=hidden_imports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
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
    name='ClickForge' if sys.platform != 'darwin' else 'ClickForge_Mac',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    icon='assets/icon.ico' if sys.platform != 'darwin' else None,
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
    name='ClickForge',
)

if sys.platform == 'darwin':
    app = BUNDLE(
        coll,
        name='ClickForge.app',
        icon=None,
        bundle_identifier='com.efeartn.clickforge',
        info_plist={
            'NSHighResolutionCapable': 'True',
            'LSBackgroundOnly': 'False',
        }
    )
