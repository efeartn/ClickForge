"""Crisp 4x-supersampled cross-platform vector icons for header controls."""

from __future__ import annotations

import math
from functools import lru_cache
from typing import Dict, Tuple

import customtkinter as ctk
from PIL import Image, ImageChops, ImageColor, ImageDraw

from clickforge.ui import theme as t

ICON_NAMES: Tuple[str, ...] = (
    "sound_on",
    "sound_off",
    "theme_system",
    "theme_light",
    "theme_dark",
    "pin_off",
    "pin_on",
    "update",
)

_CANVAS = 112  # 4x supersampling for 28x28 (@2x of 14x14 logical px)


def _rgba(hex_color: str, alpha: int = 255) -> Tuple[int, int, int, int]:
    r, g, b = ImageColor.getrgb(hex_color)
    return r, g, b, alpha


def _cap_line(
    draw: ImageDraw.ImageDraw,
    p0: Tuple[float, float],
    p1: Tuple[float, float],
    color: Tuple[int, int, int, int],
    width: int,
) -> None:
    draw.line([p0, p1], fill=color, width=width)
    r = width / 2.0
    for x, y in (p0, p1):
        draw.ellipse([x - r, y - r, x + r, y + r], fill=color)


def _draw_glyph(name: str, color_hex: str, size: int = 14) -> Image.Image:
    s = _CANVAS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    col = _rgba(color_hex)
    stroke = 9

    if name == "sound_on":
        d.polygon([(14, 42), (34, 42), (56, 20), (56, 92), (34, 70), (14, 70)], fill=col)
        d.arc([44, 34, 80, 78], start=-50, end=50, fill=col, width=stroke)
        d.arc([40, 18, 98, 94], start=-52, end=52, fill=col, width=stroke)
    elif name == "sound_off":
        d.polygon([(12, 42), (32, 42), (52, 20), (52, 92), (32, 70), (12, 70)], fill=col)
        _cap_line(d, (68, 42), (96, 70), col, stroke)
        _cap_line(d, (68, 70), (96, 42), col, stroke)
    elif name == "theme_system":
        d.ellipse([16, 16, 96, 96], outline=col, width=stroke)
        d.pieslice([16, 16, 96, 96], start=90, end=270, fill=col)
    elif name == "theme_light":
        d.ellipse([36, 36, 76, 76], outline=col, width=stroke)
        cx = cy = 56.0
        for deg in range(0, 360, 45):
            rad = math.radians(deg)
            p0 = (cx + 31.0 * math.cos(rad), cy + 31.0 * math.sin(rad))
            p1 = (cx + 42.0 * math.cos(rad), cy + 42.0 * math.sin(rad))
            _cap_line(d, p0, p1, col, 8)
    elif name == "theme_dark":
        mask = Image.new("L", (s, s), 0)
        md = ImageDraw.Draw(mask)
        md.ellipse([18, 18, 94, 94], fill=255)
        cut = Image.new("L", (s, s), 0)
        cd = ImageDraw.Draw(cut)
        cd.ellipse([44, 12, 112, 80], fill=255)
        mask = ImageChops.subtract(mask, cut)
        img.paste(col, (0, 0), mask)
    elif name in ("pin_off", "pin_on"):
        # Clean diagonal pushpin pointing bottom-left
        _cap_line(d, (54, 58), (22, 90), col, stroke)
        _cap_line(d, (38, 42), (70, 74), col, stroke)
        if name == "pin_on":
            d.polygon([(50, 48), (64, 24), (88, 48), (64, 62)], fill=col)
        else:
            d.polygon([(46, 50), (66, 22), (90, 46), (62, 66)], outline=col, width=8)
    elif name == "update":
        _cap_line(d, (56, 16), (56, 66), col, stroke)
        _cap_line(d, (36, 48), (56, 68), col, stroke)
        _cap_line(d, (76, 48), (56, 68), col, stroke)
        _cap_line(d, (22, 70), (22, 90), col, stroke)
        _cap_line(d, (22, 90), (90, 90), col, stroke)
        _cap_line(d, (90, 90), (90, 70), col, stroke)

    target = size * 2
    return img.resize((target, target), Image.Resampling.LANCZOS)


@lru_cache(maxsize=32)
def _cached_pil_pair(
    name: str, light_hex: str, dark_hex: str, size: int = 14
) -> Tuple[Image.Image, Image.Image]:
    # Cache only the pure PIL bitmaps; CTkImage instances cache PhotoImages bound
    # to a specific Tcl interpreter, so sharing them across Tk roots fails.
    return (
        _draw_glyph(name, light_hex, size=size),
        _draw_glyph(name, dark_hex, size=size),
    )


def header_icon(name: str, active: bool = False, size: int = 14) -> ctk.CTkImage:
    """Return a crisp 2x vector icon colored for light/dark themes."""
    color_pair: Tuple[str, str] = t.ACCENT if active else t.TEXT_MUTED
    light_img, dark_img = _cached_pil_pair(name, color_pair[0], color_pair[1], size=size)
    return ctk.CTkImage(
        light_image=light_img,
        dark_image=dark_img,
        size=(size, size),
    )
