"""Windows Programs installation and Setup integration for ClickForge.

Supports two installation paths without requiring UAC admin privileges:

1. **In-place portable install**: when running the unpacked Windows bundle
   outside ``%LOCALAPPDATA%\\Programs\\ClickForge``, installs into Windows
   Programs, adds Start Menu / optional Desktop shortcuts, and registers an
   entry in Windows *Installed Apps / Programs and Features*.
2. **Official Setup wizard**: downloads or launches ``ClickForge-Setup.exe``
   from the latest GitHub Release when running uninstalled or from source.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import threading
from typing import Callable, Optional

from clickforge import __app_name__, __version__
from clickforge.core import updater

IS_WINDOWS = sys.platform.startswith("win")
SETUP_ASSET_NAME = "ClickForge-Setup.exe"
UNINSTALL_REG_PATH = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ClickForge"


def default_install_dir() -> str:
    """Per-user Programs directory compatible with self-updates without UAC."""
    local_app = os.environ.get("LOCALAPPDATA") or os.path.join(
        os.path.expanduser("~"), "AppData", "Local"
    )
    return os.path.join(local_app, "Programs", __app_name__)


def start_menu_dir() -> str:
    app_data = os.environ.get("APPDATA") or os.path.join(
        os.path.expanduser("~"), "AppData", "Roaming"
    )
    return os.path.join(app_data, "Microsoft", "Windows", "Start Menu", "Programs", __app_name__)


def desktop_dir() -> str:
    user_profile = os.environ.get("USERPROFILE") or os.path.expanduser("~")
    return os.path.join(user_profile, "Desktop")


def _under_dir(path: str, parent: str) -> bool:
    try:
        a = os.path.normcase(os.path.abspath(path))
        b = os.path.normcase(os.path.abspath(parent))
        return os.path.commonpath([a, b]) == b
    except ValueError:
        return False


def is_installed_in_programs(
    exe_path: Optional[str] = None,
    frozen: Optional[bool] = None,
) -> bool:
    """Return True on non-Windows or when the frozen exe runs from Programs."""
    if not IS_WINDOWS:
        return True
    is_frozen = bool(getattr(sys, "frozen", False)) if frozen is None else frozen
    if not is_frozen:
        return False
    exe = exe_path or sys.executable
    candidates = [default_install_dir()]
    for env_var in ("ProgramFiles", "ProgramFiles(x86)"):
        root = os.environ.get(env_var)
        if root:
            candidates.append(os.path.join(root, __app_name__))
    return any(_under_dir(exe, c) for c in candidates)


def can_install_portable() -> bool:
    """True if running from a frozen Windows folder outside Programs."""
    if not IS_WINDOWS or not getattr(sys, "frozen", False):
        return False
    exe_dir = os.path.dirname(os.path.abspath(sys.executable))
    return (
        not is_installed_in_programs()
        and os.path.isdir(os.path.join(exe_dir, "_internal"))
    )


def find_local_installer() -> Optional[str]:
    """Return path to a local ``ClickForge-Setup.exe`` if already present."""
    candidates = []
    if getattr(sys, "frozen", False):
        candidates.append(os.path.join(os.path.dirname(os.path.abspath(sys.executable)), SETUP_ASSET_NAME))
    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    candidates.append(os.path.join(repo_root, "dist", SETUP_ASSET_NAME))
    for path in candidates:
        if os.path.isfile(path):
            return path
    return None


def select_setup_asset(release: updater.ReleaseInfo) -> Optional[updater.Asset]:
    """Find ``ClickForge-Setup.exe`` inside a GitHub release."""
    for asset in release.assets:
        if asset.name.lower() == SETUP_ASSET_NAME.lower():
            return asset
    return None


def _dir_size_kb(path: str) -> int:
    total = 0
    for root, _dirs, files in os.walk(path):
        for name in files:
            try:
                total += os.path.getsize(os.path.join(root, name))
            except OSError:
                pass
    return max(1, total // 1024)


def _create_shortcut(
    lnk_path: str,
    target_path: str,
    working_dir: str,
    icon_path: str,
    description: str,
) -> None:
    os.makedirs(os.path.dirname(lnk_path), exist_ok=True)
    ps_script = (
        "$sh = New-Object -ComObject WScript.Shell; "
        "$sc = $sh.CreateShortcut($env:CF_LNK); "
        "$sc.TargetPath = $env:CF_TARGET; "
        "$sc.WorkingDirectory = $env:CF_WORKDIR; "
        "$sc.IconLocation = $env:CF_ICON; "
        "$sc.Description = $env:CF_DESC; "
        "$sc.Save()"
    )
    env = os.environ.copy()
    env.update({
        "CF_LNK": lnk_path,
        "CF_TARGET": target_path,
        "CF_WORKDIR": working_dir,
        "CF_ICON": icon_path,
        "CF_DESC": description,
    })
    kwargs: dict = {"env": env, "capture_output": True, "text": True, "timeout": 15}
    if IS_WINDOWS:
        kwargs["creationflags"] = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    res = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_script],
        **kwargs,
    )
    if res.returncode != 0:
        raise RuntimeError(res.stderr.strip() or f"Shortcut creation failed ({lnk_path})")


def build_uninstall_script(
    install_dir: str,
    menu_dir: str,
    desktop_lnk: str,
) -> str:
    """Return contents of a clean per-user uninstall script for Programs."""
    return (
        "@echo off\r\n"
        "setlocal\r\n"
        "taskkill /F /IM ClickForge.exe >nul 2>&1\r\n"
        f'reg delete "HKCU\\{UNINSTALL_REG_PATH}" /f >nul 2>&1\r\n'
        f'rmdir /s /q "{menu_dir}" >nul 2>&1\r\n'
        f'del /f /q "{desktop_lnk}" >nul 2>&1\r\n'
        f'set "CF_RM={install_dir}"\r\n'
        'start "" /b powershell -NoProfile -WindowStyle Hidden -Command '
        '"Start-Sleep -Seconds 1; Remove-Item -LiteralPath $env:CF_RM -Recurse -Force -ErrorAction SilentlyContinue"\r\n'
        "exit /b 0\r\n"
    )


def _register_uninstall_entry(
    install_dir: str,
    exe_path: str,
    uninstall_cmd: str,
    size_kb: int,
) -> None:
    import winreg

    with winreg.CreateKeyEx(
        winreg.HKEY_CURRENT_USER, UNINSTALL_REG_PATH, 0, winreg.KEY_WRITE
    ) as key:
        values = {
            "DisplayName": (__app_name__, winreg.REG_SZ),
            "DisplayVersion": (__version__, winreg.REG_SZ),
            "Publisher": ("efeartn", winreg.REG_SZ),
            "URLInfoAbout": (f"https://github.com/{updater.GITHUB_REPO}", winreg.REG_SZ),
            "InstallLocation": (install_dir, winreg.REG_SZ),
            "DisplayIcon": (exe_path, winreg.REG_SZ),
            "UninstallString": (f'"{uninstall_cmd}"', winreg.REG_SZ),
            "QuietUninstallString": (f'"{uninstall_cmd}"', winreg.REG_SZ),
            "EstimatedSize": (int(size_kb), winreg.REG_DWORD),
            "NoModify": (1, winreg.REG_DWORD),
            "NoRepair": (1, winreg.REG_DWORD),
        }
        for name, (val, reg_type) in values.items():
            winreg.SetValueEx(key, name, 0, reg_type, val)


def install_portable_bundle(
    install_dir: Optional[str] = None,
    create_desktop_shortcut: bool = True,
) -> str:
    """Install the running portable bundle into Windows Programs & Start Menu.

    Returns the path to the installed ``ClickForge.exe``.
    """
    if not IS_WINDOWS:
        raise RuntimeError("Windows installation is only supported on Windows.")
    if not getattr(sys, "frozen", False):
        raise RuntimeError("Self-install requires a packaged ClickForge.exe bundle.")

    src_dir = os.path.dirname(os.path.abspath(sys.executable))
    if not os.path.isdir(os.path.join(src_dir, "_internal")):
        raise RuntimeError("Incomplete application folder (_internal missing).")

    dest_dir = os.path.abspath(install_dir or default_install_dir())
    os.makedirs(os.path.dirname(dest_dir), exist_ok=True)
    if os.path.normcase(src_dir) != os.path.normcase(dest_dir):
        if os.path.exists(dest_dir):
            shutil.rmtree(dest_dir, ignore_errors=False)
        shutil.copytree(src_dir, dest_dir)

    exe_name = os.path.basename(sys.executable)
    installed_exe = os.path.join(dest_dir, exe_name)
    menu_dir = start_menu_dir()
    menu_lnk = os.path.join(menu_dir, f"{__app_name__}.lnk")
    uninstall_lnk = os.path.join(menu_dir, f"Uninstall {__app_name__}.lnk")
    desk_lnk = os.path.join(desktop_dir(), f"{__app_name__}.lnk")
    uninstall_cmd = os.path.join(dest_dir, f"Uninstall {__app_name__}.cmd")

    with open(uninstall_cmd, "w", encoding="utf-8") as f:
        f.write(build_uninstall_script(dest_dir, menu_dir, desk_lnk))

    _create_shortcut(
        menu_lnk,
        installed_exe,
        dest_dir,
        f"{installed_exe},0",
        "High-precision desktop auto-clicker",
    )
    _create_shortcut(
        uninstall_lnk,
        uninstall_cmd,
        dest_dir,
        f"{installed_exe},0",
        f"Remove {__app_name__} from Windows",
    )
    if create_desktop_shortcut:
        _create_shortcut(
            desk_lnk,
            installed_exe,
            dest_dir,
            f"{installed_exe},0",
            "High-precision desktop auto-clicker",
        )

    _register_uninstall_entry(dest_dir, installed_exe, uninstall_cmd, _dir_size_kb(dest_dir))
    return installed_exe


def launch_exe(exe_path: str) -> None:
    """Start an executable or installer detached from this process."""
    if not os.path.isfile(exe_path):
        raise FileNotFoundError(exe_path)
    if IS_WINDOWS:
        os.startfile(exe_path)  # noqa: S606 - user-initiated launch of local installer/exe
    else:
        subprocess.Popen([exe_path], start_new_session=True)


def download_and_launch_setup(
    progress: Optional[Callable[[int, int], None]] = None,
    cancel: Optional[threading.Event] = None,
) -> str:
    """Download ``ClickForge-Setup.exe`` from the latest release and run it."""
    release = updater.fetch_latest()
    asset = select_setup_asset(release)
    if asset is None:
        raise updater.UpdateError(
            f"Release v{release.version_text} does not include {SETUP_ASSET_NAME}."
        )
    dest_dir = tempfile.mkdtemp(prefix="clickforge-setup-")
    setup_path = updater.download_asset(
        asset, dest_dir, progress=progress, cancel=cancel
    )
    launch_exe(setup_path)
    return setup_path
