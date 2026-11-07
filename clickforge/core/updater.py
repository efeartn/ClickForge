"""Auto-updater backed by GitHub Releases.

Flow
----
1. :func:`fetch_latest` asks the GitHub API for the newest published release.
2. :meth:`ReleaseInfo.asset_for_platform` picks the matching archive
   (``ClickForge-Windows.zip`` / ``-macOS.zip`` / ``-Linux.zip``).
3. :func:`download_asset` streams it to a temp folder and verifies both the
   byte size and the SHA-256 digest GitHub publishes for every asset.
4. On Windows (frozen builds only) :func:`stage_update` extracts it safely and
   :func:`launch_installer` hands off to a tiny helper script that waits for
   ClickForge to exit, swaps the files with rollback on failure, and relaunches
   the new version.  Other platforms open the release page instead.

Nothing here touches the UI; callers run these functions on a worker thread.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import ssl
import subprocess
import sys
import tempfile
import threading
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass
from typing import Callable, Optional, Tuple

from clickforge import __version__

REPO = "efeartn/ClickForge"
API_LATEST = f"https://api.github.com/repos/{REPO}/releases/latest"
RELEASES_PAGE = f"https://github.com/{REPO}/releases/latest"

IS_WINDOWS = sys.platform.startswith("win")
IS_MAC = sys.platform == "darwin"

ASSET_NAMES = {
    "windows": "ClickForge-Windows.zip",
    "macos": "ClickForge-macOS.zip",
    "linux": "ClickForge-Linux.zip",
}

#: Hard ceiling on download size; anything bigger is not a ClickForge build.
MAX_DOWNLOAD_BYTES = 300 * 1024 * 1024

Version = Tuple[int, int, int]


class UpdateError(Exception):
    """Raised with a short, user-presentable message."""


# ── Versions ─────────────────────────────────────────────────────────


def parse_version(text: object) -> Optional[Version]:
    """Parse ``"v2.3.0"``, ``"2.3"`` or ``"2.3.0-beta"`` into ``(2, 3, 0)``."""
    m = re.match(r"^\s*v?(\d+)(?:\.(\d+))?(?:\.(\d+))?", str(text or ""))
    if not m:
        return None
    return int(m.group(1)), int(m.group(2) or 0), int(m.group(3) or 0)


def current_version() -> Version:
    return parse_version(__version__) or (0, 0, 0)


def format_version(v: Version) -> str:
    return "{}.{}.{}".format(*v)


def is_newer(remote: Optional[Version], local: Optional[Version] = None) -> bool:
    if remote is None:
        return False
    return remote > (local or current_version())


def platform_key() -> str:
    if IS_WINDOWS:
        return "windows"
    if IS_MAC:
        return "macos"
    return "linux"


# ── Release metadata ─────────────────────────────────────────────────


@dataclass(frozen=True)
class Asset:
    name: str
    url: str
    size: int
    sha256: Optional[str] = None


@dataclass(frozen=True)
class ReleaseInfo:
    tag: str
    version: Version
    title: str
    page_url: str
    notes: str
    assets: Tuple[Asset, ...]

    @property
    def version_text(self) -> str:
        return format_version(self.version)

    def asset_for_platform(self, key: Optional[str] = None) -> Optional[Asset]:
        wanted = ASSET_NAMES[key or platform_key()]
        for asset in self.assets:
            if asset.name == wanted:
                return asset
        return None


def _ssl_context() -> ssl.SSLContext:
    # Windows: the system certificate store (respects corporate root CAs).
    # Elsewhere frozen Pythons often lack a CA bundle, so prefer certifi.
    if not IS_WINDOWS:
        try:
            import certifi  # type: ignore

            return ssl.create_default_context(cafile=certifi.where())
        except Exception:
            pass
    return ssl.create_default_context()


def _request(url: str, accept: str) -> urllib.request.Request:
    return urllib.request.Request(
        url,
        headers={
            "Accept": accept,
            "User-Agent": f"ClickForge/{__version__} (+https://github.com/{REPO})",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )


def parse_release(data: dict) -> ReleaseInfo:
    """Build a :class:`ReleaseInfo` from a GitHub ``/releases`` payload."""
    tag = str(data.get("tag_name") or "")
    version = parse_version(tag)
    if version is None:
        raise UpdateError(f"Unrecognised release tag '{tag}'.")
    assets = []
    for raw in data.get("assets") or ():
        digest = str(raw.get("digest") or "")
        sha = digest.split(":", 1)[1].lower() if digest.startswith("sha256:") else None
        assets.append(
            Asset(
                name=str(raw.get("name") or ""),
                url=str(raw.get("browser_download_url") or ""),
                size=int(raw.get("size") or 0),
                sha256=sha,
            )
        )
    return ReleaseInfo(
        tag=tag,
        version=version,
        title=str(data.get("name") or tag),
        page_url=str(data.get("html_url") or RELEASES_PAGE),
        notes=str(data.get("body") or ""),
        assets=tuple(assets),
    )


def fetch_latest(timeout: float = 8.0) -> ReleaseInfo:
    """Return the newest published (non-draft, non-prerelease) release."""
    try:
        with urllib.request.urlopen(
            _request(API_LATEST, "application/vnd.github+json"),
            timeout=timeout,
            context=_ssl_context(),
        ) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        if exc.code == 403:
            raise UpdateError("GitHub rate limit reached. Try again later.") from exc
        if exc.code == 404:
            raise UpdateError("No published releases found.") from exc
        raise UpdateError(f"GitHub returned HTTP {exc.code}.") from exc
    except (urllib.error.URLError, OSError, TimeoutError) as exc:
        raise UpdateError("Could not reach GitHub. Check your connection.") from exc
    except ValueError as exc:
        raise UpdateError("GitHub sent an unreadable response.") from exc
    return parse_release(data)


# ── Download ─────────────────────────────────────────────────────────


def download_asset(
    asset: Asset,
    dest_dir: str,
    progress: Optional[Callable[[int, int], None]] = None,
    cancel: Optional[threading.Event] = None,
    timeout: float = 20.0,
) -> str:
    """Stream ``asset`` into ``dest_dir`` and verify size + SHA-256."""
    if not asset.url.startswith("https://"):
        raise UpdateError("Refusing to download over an insecure connection.")
    if asset.size > MAX_DOWNLOAD_BYTES:
        raise UpdateError("Release archive is unexpectedly large.")
    os.makedirs(dest_dir, exist_ok=True)
    path = os.path.join(dest_dir, asset.name)
    digest = hashlib.sha256()
    received = 0
    try:
        with urllib.request.urlopen(
            _request(asset.url, "application/octet-stream"),
            timeout=timeout,
            context=_ssl_context(),
        ) as resp, open(path, "wb") as out:
            total = asset.size or int(resp.headers.get("Content-Length") or 0)
            while True:
                if cancel is not None and cancel.is_set():
                    raise UpdateError("Download cancelled.")
                chunk = resp.read(64 * 1024)
                if not chunk:
                    break
                received += len(chunk)
                if received > MAX_DOWNLOAD_BYTES:
                    raise UpdateError("Release archive is unexpectedly large.")
                digest.update(chunk)
                out.write(chunk)
                if progress is not None:
                    progress(received, total)
    except UpdateError:
        _silent_remove(path)
        raise
    except (urllib.error.URLError, OSError, TimeoutError) as exc:
        _silent_remove(path)
        raise UpdateError("Download failed. Check your connection.") from exc

    if asset.size and received != asset.size:
        _silent_remove(path)
        raise UpdateError("Download was incomplete.")
    if asset.sha256 and digest.hexdigest() != asset.sha256:
        _silent_remove(path)
        raise UpdateError("Checksum mismatch; the download was discarded.")
    return path


def _silent_remove(path: str) -> None:
    try:
        os.remove(path)
    except OSError:
        pass


# ── Install (Windows, frozen builds) ─────────────────────────────────


def install_dir() -> Optional[str]:
    """Folder containing the running ``ClickForge.exe`` (frozen builds only)."""
    if not getattr(sys, "frozen", False):
        return None
    return os.path.dirname(os.path.abspath(sys.executable))


def can_self_update() -> Tuple[bool, str]:
    """Whether the running build can replace itself in place."""
    if not IS_WINDOWS:
        return False, "In-place updates are available on Windows; opening the download page."
    target = install_dir()
    if target is None:
        return False, "Running from source. Use 'git pull' to update."
    if not os.path.isdir(os.path.join(target, "_internal")):
        return False, "This build layout cannot be updated in place."
    try:
        fd, probe = tempfile.mkstemp(dir=target, prefix=".cf-write-test-")
        os.close(fd)
        os.remove(probe)
    except OSError:
        return False, "ClickForge's folder is read-only. Download the update manually."
    return True, ""


def safe_extract(zip_path: str, dest_dir: str) -> None:
    """Extract ``zip_path`` refusing absolute paths and ``..`` traversal."""
    root = os.path.realpath(dest_dir)
    try:
        with zipfile.ZipFile(zip_path) as zf:
            for member in zf.infolist():
                name = member.filename.replace("\\", "/")
                target = os.path.realpath(os.path.join(root, name))
                if os.path.isabs(name) or not (target == root or target.startswith(root + os.sep)):
                    raise UpdateError("Archive contains an unsafe path.")
            zf.extractall(root)
    except zipfile.BadZipFile as exc:
        raise UpdateError("Downloaded archive is corrupted.") from exc


def find_app_root(extracted: str, exe_name: str = "ClickForge.exe") -> Optional[str]:
    """Locate the folder holding ``exe_name`` + ``_internal`` (max depth 2)."""
    candidates = [extracted]
    try:
        candidates += [
            os.path.join(extracted, d)
            for d in sorted(os.listdir(extracted))
            if os.path.isdir(os.path.join(extracted, d))
        ]
    except OSError:
        return None
    for folder in candidates:
        if os.path.isfile(os.path.join(folder, exe_name)) and os.path.isdir(
            os.path.join(folder, "_internal")
        ):
            return folder
    return None


def stage_update(zip_path: str, workdir: str) -> str:
    """Extract the archive into ``workdir/staged`` and return the app folder."""
    staged = os.path.join(workdir, "staged")
    shutil.rmtree(staged, ignore_errors=True)
    os.makedirs(staged)
    safe_extract(zip_path, staged)
    app_root = find_app_root(staged)
    if app_root is None:
        raise UpdateError("Archive does not contain a ClickForge build.")
    return app_root


# The helper runs after ClickForge exits.  All paths arrive through environment
# variables (Unicode-safe, no quoting issues), so this script stays pure ASCII.
_INSTALLER_CMD = r"""@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "LOG=%CF_WORK%\update.log"
echo ClickForge updater started > "%LOG%"
set /a TRIES=0

:wait
tasklist /FI "PID eq %CF_PID%" /FO CSV /NH 2>nul | findstr /R /C:",.%CF_PID%.," >nul
if errorlevel 1 goto swap
set /a TRIES+=1
if %TRIES% GEQ 90 goto give_up
ping -n 2 127.0.0.1 >nul
goto wait

:swap
echo Replacing files in %CF_DST% >> "%LOG%"
if exist "%CF_DST%\_internal.cfbak" rmdir /s /q "%CF_DST%\_internal.cfbak"
move "%CF_DST%\_internal" "%CF_DST%\_internal.cfbak" >> "%LOG%" 2>&1
if errorlevel 1 goto give_up
copy /y "%CF_DST%\%CF_EXE%" "%CF_DST%\%CF_EXE%.cfbak" >> "%LOG%" 2>&1
robocopy "%CF_SRC%\_internal" "%CF_DST%\_internal" /E /R:5 /W:1 /NFL /NDL /NJH /NJS /NP >> "%LOG%" 2>&1
if errorlevel 8 goto rollback
copy /y "%CF_SRC%\ClickForge.exe" "%CF_DST%\%CF_EXE%" >> "%LOG%" 2>&1
if errorlevel 1 goto rollback
rmdir /s /q "%CF_DST%\_internal.cfbak" >nul 2>&1
del /f /q "%CF_DST%\%CF_EXE%.cfbak" >nul 2>&1
echo Update installed >> "%LOG%"
start "" /D "%CF_DST%" "%CF_DST%\%CF_EXE%"
rmdir /s /q "%CF_WORK%\staged" >nul 2>&1
del /f /q "%CF_WORK%\*.zip" >nul 2>&1
goto done

:rollback
echo Install failed, rolling back >> "%LOG%"
rmdir /s /q "%CF_DST%\_internal" >nul 2>&1
move "%CF_DST%\_internal.cfbak" "%CF_DST%\_internal" >> "%LOG%" 2>&1
copy /y "%CF_DST%\%CF_EXE%.cfbak" "%CF_DST%\%CF_EXE%" >> "%LOG%" 2>&1
del /f /q "%CF_DST%\%CF_EXE%.cfbak" >nul 2>&1

:give_up
echo Update aborted >> "%LOG%"
start "" /D "%CF_DST%" "%CF_DST%\%CF_EXE%"

:done
endlocal
(goto) 2>nul & del "%~f0"
"""


def launch_installer(staged_app: str, workdir: str) -> None:
    """Start the detached helper that swaps files once this process exits."""
    target = install_dir()
    if not IS_WINDOWS or target is None:
        raise UpdateError("In-place install is not supported here.")
    script = os.path.join(workdir, "apply_update.cmd")
    with open(script, "w", encoding="ascii", newline="\r\n") as f:
        f.write(_INSTALLER_CMD)
    env = dict(os.environ)
    env.update(
        CF_PID=str(os.getpid()),
        CF_SRC=staged_app,
        CF_DST=target,
        CF_EXE=os.path.basename(sys.executable),
        CF_WORK=workdir,
    )
    # Never leak PyInstaller's runtime paths into the relaunched process.
    for key in [k for k in env if k.startswith("_PYI") or k == "_MEIPASS2"]:
        env.pop(key, None)
    flags = subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP
    breakaway = 0x01000000  # CREATE_BREAKAWAY_FROM_JOB
    cmd = [os.environ.get("COMSPEC", "cmd.exe"), "/d", "/c", script]
    parent_tmp = tempfile.gettempdir()
    try:
        subprocess.Popen(
            cmd, env=env, cwd=parent_tmp, creationflags=flags | breakaway, close_fds=True
        )
    except OSError:
        subprocess.Popen(cmd, env=env, cwd=parent_tmp, creationflags=flags, close_fds=True)


def new_workdir(version: Version) -> str:
    path = os.path.join(tempfile.gettempdir(), f"ClickForge-update-{format_version(version)}")
    shutil.rmtree(path, ignore_errors=True)
    os.makedirs(path, exist_ok=True)
    return path
