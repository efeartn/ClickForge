"""Subtle audio cues for auto-clicking state changes.

Plays short, non-intrusive chimes when clicking starts and stops so users
receive instant auditory feedback even when their target window or game is
fullscreen.

All audio playback is executed on a dedicated background thread to guarantee
zero latency impact on the input injection worker or UI thread.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import threading
from typing import List, Optional

IS_WINDOWS = sys.platform.startswith("win")
IS_MAC = sys.platform == "darwin"

_LINUX_SOUND_FILE = "/usr/share/sounds/freedesktop/stereo/message.oga"
_LINUX_PLAYER: Optional[List[str]] = None
_LINUX_PLAYER_CHECKED = False


def _find_linux_player() -> Optional[List[str]]:
    global _LINUX_PLAYER, _LINUX_PLAYER_CHECKED
    if _LINUX_PLAYER_CHECKED:
        return _LINUX_PLAYER
    _LINUX_PLAYER_CHECKED = True
    if shutil.which("canberra-gtk-play"):
        _LINUX_PLAYER = ["canberra-gtk-play", "-i", "message"]
    elif shutil.which("paplay") and os.path.isfile(_LINUX_SOUND_FILE):
        _LINUX_PLAYER = ["paplay", _LINUX_SOUND_FILE]
    return _LINUX_PLAYER


def _play_worker(freq: int, duration_ms: int) -> None:
    if IS_WINDOWS:
        try:
            import winsound

            winsound.Beep(freq, duration_ms)
        except Exception:
            pass
    elif IS_MAC:
        try:
            sound_file = (
                "/System/Library/Sounds/Tink.aiff"
                if freq > 1000
                else "/System/Library/Sounds/Pop.aiff"
            )
            subprocess.run(
                ["afplay", sound_file],
                timeout=1.5,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass
    else:
        cmd = _find_linux_player()
        if cmd:
            try:
                subprocess.run(
                    cmd,
                    timeout=1.5,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            except Exception:
                pass


def play_cue(event: str, enabled: bool = True) -> None:
    """Play a short audio cue asynchronously.
    
    :param event: ``"start"`` for higher start chime, ``"stop"`` for lower stop chime.
    :param enabled: if ``False``, this is a no-op.
    """
    if not enabled:
        return

    # High crisp chime (1320 Hz, 35 ms) on start; gentle low chime (880 Hz, 35 ms) on stop
    freq = 1320 if event == "start" else 880
    threading.Thread(
        target=_play_worker,
        args=(freq, 35),
        name="ClickForge-AudioCue",
        daemon=True,
    ).start()
