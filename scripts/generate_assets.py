"""Deterministic generator for ClickForge brand and icon assets.

Creates an original minimalist geometric logo:
- Dark squircle background (#111118) with a subtle 1px border (#262636)
- Precision pulse ring (#7C3AED) + radial spark ticks at the click origin
- Two-tone faceted geometric cursor arrow (#FFFFFF left facet, #A78BFA right facet)

Outputs:
- assets/logo.svg   (scalable vector for README and docs)
- assets/icon.png   (512x512 high-resolution PNG)
- assets/icon.ico   (multi-DPI Windows icon: 16, 24, 32, 48, 64, 128, 256)
"""

from __future__ import annotations

import math
import os
from typing import List, Tuple

from PIL import Image, ImageDraw

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS_DIR = os.path.join(ROOT_DIR, "assets")

BG_HEX = "#111118"
BORDER_HEX = "#272736"
RING_HEX = "#7C3AED"
FACET_LIGHT_HEX = "#FFFFFF"
FACET_ACCENT_HEX = "#A78BFA"

BG_RGBA = (17, 17, 24, 255)
BORDER_RGBA = (39, 39, 54, 255)
RING_RGBA = (124, 58, 237, 255)
RING_GLOW_RGBA = (124, 58, 237, 72)
FACET_LIGHT_RGBA = (255, 255, 255, 255)
FACET_ACCENT_RGBA = (167, 139, 250, 255)
SHADOW_RGBA = (6, 6, 10, 140)

ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]

# Normalized coordinates on a 512x512 canvas:
# Cursor tip sits at (172, 164), pointing up-left into the pulse origin.
TIP = (172.0, 164.0)
LEFT_WING = (210.0, 412.0)
CENTER_NOTCH = (274.0, 332.0)
RIGHT_WING = (410.0, 244.0)


def _scale_pt(pt: Tuple[float, float], scale: float) -> Tuple[float, float]:
    return (pt[0] * scale, pt[1] * scale)


def generate_svg(path: str) -> None:
    """Write a crisp, standalone SVG version of the ClickForge mark."""
    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" fill="none">
  <!-- Dark Squircle Container -->
  <rect x="24" y="24" width="464" height="464" rx="112" fill="{BG_HEX}" stroke="{BORDER_HEX}" stroke-width="6"/>

  <!-- Precision Pulse Ring & Click Sparks at Impact Point -->
  <circle cx="172" cy="164" r="78" stroke="{RING_HEX}" stroke-width="16" stroke-opacity="0.32"/>
  <path d="M 103 200 A 78 78 0 1 1 218 101" stroke="{RING_HEX}" stroke-width="18" stroke-linecap="round"/>
  <line x1="92" y1="84" x2="118" y2="110" stroke="{FACET_ACCENT_HEX}" stroke-width="14" stroke-linecap="round"/>
  <line x1="172" y1="60" x2="172" y2="92" stroke="{RING_HEX}" stroke-width="14" stroke-linecap="round"/>
  <line x1="68" y1="164" x2="100" y2="164" stroke="{RING_HEX}" stroke-width="14" stroke-linecap="round"/>

  <!-- Subtle Elevation Shadow -->
  <polygon points="178,174 216,422 280,342 416,254" fill="#06060A" fill-opacity="0.45"/>

  <!-- Two-Tone Geometric Faceted Cursor Arrow -->
  <!-- Upper-Right Facet (#A78BFA) -->
  <polygon points="172,164 410,244 274,332" fill="{FACET_ACCENT_HEX}"/>
  <!-- Lower-Left Primary Facet (#FFFFFF) -->
  <polygon points="172,164 274,332 210,412" fill="{FACET_LIGHT_HEX}"/>

  <!-- Impact Core Dot -->
  <circle cx="172" cy="164" r="10" fill="{FACET_LIGHT_HEX}"/>
</svg>
"""
    with open(path, "w", encoding="utf-8") as f:
        f.write(svg)


def render_icon(size: int) -> Image.Image:
    """Render the geometric icon at ``size`` using 4x supersampling.

    At very small sizes (16x16, 24x24), fine details are simplified and stroke
    weights are boosted so the silhouette stays unmistakable in the titlebar.
    """
    ss = 4 if size < 256 else 2
    canvas_px = size * ss
    scale = canvas_px / 512.0
    small = size <= 24

    img = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Squircle background
    pad = int(round((16 if small else 24) * scale))
    radius = int(round((116 if small else 112) * scale))
    border_w = max(1, int(round((10 if small else 6) * scale)))
    draw.rounded_rectangle(
        [pad, pad, canvas_px - pad - 1, canvas_px - pad - 1],
        radius=radius,
        fill=BG_RGBA,
        outline=BORDER_RGBA,
        width=border_w,
    )

    # 2. Precision pulse ring around the click point
    tx, ty = _scale_pt(TIP, scale)
    ring_r = (86.0 if small else 78.0) * scale
    ring_w = max(2, int(round((28.0 if small else 18.0) * scale)))
    bbox = [tx - ring_r, ty - ring_r, tx + ring_r, ty + ring_r]

    if not small:
        draw.ellipse(bbox, outline=RING_GLOW_RGBA, width=max(2, int(round(16 * scale))))

    # Active pulse arc (top-left 230 degrees around click point)
    draw.arc(bbox, start=145, end=330, fill=RING_RGBA, width=ring_w)

    # Radial precision spark ticks (omitted at 16x16 to prevent muddiness)
    if size >= 24:
        spark_w = max(2, int(round((18 if small else 14) * scale)))
        for angle_deg, color in ((225, FACET_ACCENT_RGBA), (270, RING_RGBA), (180, RING_RGBA)):
            rad = math.radians(angle_deg)
            r_in = ring_r * 1.05
            r_out = ring_r * 1.34
            x0 = tx + math.cos(rad) * r_in
            y0 = ty + math.sin(rad) * r_in
            x1 = tx + math.cos(rad) * r_out
            y1 = ty + math.sin(rad) * r_out
            draw.line([(x0, y0), (x1, y1)], fill=color, width=spark_w)

    # 3. Two-tone faceted cursor arrow
    if small:
        # Slightly enlarged cursor at 16/24px for maximum legibility
        tip = _scale_pt((158.0, 150.0), scale)
        left_wing = _scale_pt((204.0, 428.0), scale)
        notch = _scale_pt((278.0, 340.0), scale)
        right_wing = _scale_pt((426.0, 240.0), scale)
    else:
        tip = _scale_pt(TIP, scale)
        left_wing = _scale_pt(LEFT_WING, scale)
        notch = _scale_pt(CENTER_NOTCH, scale)
        right_wing = _scale_pt(RIGHT_WING, scale)

        # Subtle drop shadow
        sh_off = 8.0 * scale
        draw.polygon(
            [
                (tip[0] + sh_off, tip[1] + sh_off),
                (left_wing[0] + sh_off, left_wing[1] + sh_off),
                (notch[0] + sh_off, notch[1] + sh_off),
                (right_wing[0] + sh_off, right_wing[1] + sh_off),
            ],
            fill=SHADOW_RGBA,
        )

    # Upper-right facet (#A78BFA)
    draw.polygon([tip, right_wing, notch], fill=FACET_ACCENT_RGBA)
    # Lower-left facet (#FFFFFF)
    draw.polygon([tip, notch, left_wing], fill=FACET_LIGHT_RGBA)

    return img.resize((size, size), Image.Resampling.LANCZOS)


def main() -> None:
    os.makedirs(ASSETS_DIR, exist_ok=True)

    svg_path = os.path.join(ASSETS_DIR, "logo.svg")
    png_path = os.path.join(ASSETS_DIR, "icon.png")
    ico_path = os.path.join(ASSETS_DIR, "icon.ico")

    generate_svg(svg_path)

    icon_512 = render_icon(512)
    icon_512.save(png_path, format="PNG", optimize=True)

    frames: List[Image.Image] = [render_icon(s) for s in ICO_SIZES]
    # Save largest frame as primary with explicit multi-DPI frames appended
    frames[-1].save(
        ico_path,
        format="ICO",
        append_images=frames[:-1],
        sizes=[(s, s) for s in ICO_SIZES],
    )
    print(f"Generated: {svg_path}")
    print(f"Generated: {png_path} (512x512)")
    print(f"Generated: {ico_path} (frames: {ICO_SIZES})")


if __name__ == "__main__":
    main()
