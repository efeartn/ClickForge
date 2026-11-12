"""Design tokens: colors, typography and icon glyphs.

Colors are ``(light, dark)`` tuples as expected by CustomTkinter. The palette
is an Apple-inspired minimalist dark/light system: subtle neutral surfaces,
refined specular borders, and a purposeful royal violet accent reserved for
primary actions and active focus states.
"""

from __future__ import annotations

import sys
import tkinter.font as tkfont
from dataclasses import dataclass
from typing import Iterable, Optional

# ── Surfaces ─────────────────────────────────────────────────────────
BG = ("#F5F5F7", "#0C0C10")
SURFACE = ("#FFFFFF", "#15151B")
FIELD = ("#EEEEF1", "#1D1D24")
FIELD_DISABLED = ("#F5F5F7", "#131318")
BORDER = ("#E2E2E8", "#262633")
BORDER_STRONG = ("#CECED6", "#363646")
HOVER = ("#E8E8ED", "#23232D")
SEG_SELECTED = ("#FFFFFF", "#2D2D3A")
SEG_SELECTED_HOVER = ("#FFFFFF", "#363646")

# ── Text ─────────────────────────────────────────────────────────────
TEXT = ("#18181B", "#EDEDF0")
TEXT_MUTED = ("#5B5B66", "#A1A1AC")
TEXT_FAINT = ("#9A9AA3", "#606070")
TEXT_ON_ACCENT = ("#FFFFFF", "#FFFFFF")

# ── Semantic ─────────────────────────────────────────────────────────
ACCENT = ("#7C3AED", "#7C3AED")
ACCENT_HOVER = ("#6D28D9", "#8B5CF6")
ACCENT_DISABLED = ("#D9D2EC", "#29243C")
DANGER = ("#DC2626", "#E5484D")
DANGER_HOVER = ("#B91C1C", "#F06A6E")
SUCCESS = ("#16A34A", "#3DD68C")
SUCCESS_SOFT = ("#E8F7EE", "#11241A")
WARNING = ("#D97706", "#F5A524")

# ── Metrics ──────────────────────────────────────────────────────────
RADIUS = 10
RADIUS_SM = 7
CONTROL_H = 32
PAD = 16
GAP = 12

IS_WINDOWS = sys.platform.startswith("win")
IS_MAC = sys.platform == "darwin"


@dataclass(frozen=True)
class Fonts:
    ui: str
    ui_display: str
    mono: str
    icons: Optional[str]


def _first_available(candidates: Iterable[str], available: set, fallback: str) -> str:
    for name in candidates:
        if name in available:
            return name
    return fallback


def resolve_fonts(root) -> Fonts:
    """Pick the best installed families. Requires an existing Tk root."""
    available = set(tkfont.families(root))
    if IS_WINDOWS:
        ui = ["Segoe UI Variable Text", "Segoe UI"]
        display = ["Segoe UI Variable Display", "Segoe UI Variable Text", "Segoe UI"]
        mono = ["Cascadia Mono", "JetBrains Mono", "Consolas"]
    elif IS_MAC:
        ui = ["SF Pro Text", ".AppleSystemUIFont", "Helvetica Neue"]
        display = ["SF Pro Display", ".AppleSystemUIFont", "Helvetica Neue"]
        mono = ["SF Mono", "JetBrains Mono", "Menlo"]
    else:
        ui = ["Inter", "Cantarell", "Noto Sans", "DejaVu Sans"]
        display = ui
        mono = ["JetBrains Mono", "Cascadia Mono", "DejaVu Sans Mono", "Monospace"]
    icons = _first_available(["Segoe Fluent Icons", "Segoe MDL2 Assets"], available, "") or None
    return Fonts(
        ui=_first_available(ui, available, "TkDefaultFont"),
        ui_display=_first_available(display, available, "TkDefaultFont"),
        mono=_first_available(mono, available, "TkFixedFont"),
        icons=icons,
    )


# Segoe Fluent Icons / MDL2 code points.
GLYPH_PIN = "\uE718"
GLYPH_PINNED = "\uE840"
GLYPH_THEME = {"System": "\uE7F4", "Light": "\uE706", "Dark": "\uE708"}
GLYPH_RESET = "\uE72C"
GLYPH_UPDATE = "\uE896"       # Download arrow
GLYPH_SOUND_ON = "\uE767"     # Volume speaker
GLYPH_SOUND_OFF = "\uE74F"    # Mute speaker
