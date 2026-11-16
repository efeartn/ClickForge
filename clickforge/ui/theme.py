"""Design tokens: colors, typography and icon glyphs.

Colors are ``(light, dark)`` tuples as expected by CustomTkinter. The palette
uses neutral carbon/graphite surfaces, crisp 1px borders, and a restrained
precision-emerald accent reserved for primary actions and active states.
"""

from __future__ import annotations

import sys
import tkinter.font as tkfont
from dataclasses import dataclass
from typing import Iterable, Optional

# ── Surfaces ─────────────────────────────────────────────────────────
BG = ("#F3F4F6", "#0A0B0D")
SURFACE = ("#FFFFFF", "#121418")
FIELD = ("#EBEDF0", "#171A20")
FIELD_DISABLED = ("#F4F5F7", "#0F1115")
BORDER = ("#E2E5EA", "#22262E")
BORDER_STRONG = ("#C8CDD6", "#303642")
HOVER = ("#E4E7EC", "#1D2026")
SEG_SELECTED = ("#FFFFFF", "#262B35")
SEG_SELECTED_HOVER = ("#F9FAFB", "#2E3440")

# ── Text ─────────────────────────────────────────────────────────────
TEXT = ("#111318", "#ECEDEF")
TEXT_MUTED = ("#525866", "#949BA6")
TEXT_FAINT = ("#8E95A2", "#5A606B")
TEXT_ON_ACCENT = ("#FFFFFF", "#FFFFFF")

# ── Semantic ─────────────────────────────────────────────────────────
ACCENT = ("#059669", "#10A37F")
ACCENT_HOVER = ("#047857", "#0E8F6F")
ACCENT_DISABLED = ("#D1FAE5", "#142923")
DANGER = ("#DC2626", "#E5484D")
DANGER_HOVER = ("#B91C1C", "#F06A6E")
SUCCESS = ("#059669", "#10B981")
SUCCESS_SOFT = ("#ECFDF5", "#0D241C")
WARNING = ("#D97706", "#F5A524")

# ── Metrics ──────────────────────────────────────────────────────────
RADIUS = 10
RADIUS_SM = 7
CONTROL_H = 32
PAD = 14
GAP = 10

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
