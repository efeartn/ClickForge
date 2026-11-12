"""Deterministic generator for ClickForge brand and icon assets.

Apple-inspired minimalist design language:
- Deep obsidian-to-graphite vertical gradient squircle (#1C1C26 -> #0B0B10)
  with a subtle top-lit specular border (#343448 -> #1E1E2A)
- Soft radial ambient violet glow + concentric precision touch rings (#7C3AED / #A78BFA)
- Iconic sculpted chevron pointer (#FFFFFF) with a deep inner notch and soft elevation shadow

Outputs:
- assets/logo.svg   (scalable vector for README and docs)
- assets/icon.png   (512x512 high-resolution PNG)
- assets/icon.ico   (multi-DPI Windows icon: 16, 24, 32, 48, 64, 128, 256)
- assets/icon.icns  (macOS app bundle icon)
"""

from __future__ import annotations

import os
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFilter

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS_DIR = os.path.join(ROOT_DIR, "assets")

BG_TOP_RGB = (28, 28, 38)        # #1C1C26
BG_BOT_RGB = (11, 11, 16)        # #0B0B10
BORDER_RGBA = (52, 52, 72, 255)  # #343448

ACCENT_HEX = "#7C3AED"
ACCENT_LIGHT_HEX = "#A78BFA"
WHITE_HEX = "#FFFFFF"

AMBIENT_GLOW_RGBA = (124, 58, 237, 52)
OUTER_RING_RGBA = (124, 58, 237, 92)
INNER_RING_RGBA = (139, 92, 246, 245)
POINTER_RGBA = (255, 255, 255, 255)
SHADOW_RGBA = (0, 0, 0, 175)

ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]

# Normalized coordinates on a 512x512 canvas:
# Concentric ring origin sits at (186, 180); sculpted chevron pointer meets the origin.
ORIGIN = (186.0, 180.0)
POINTER_PTS: List[Tuple[float, float]] = [
    (186.0, 180.0),  # Tip at click origin
    (228.0, 414.0),  # Bottom-left wing
    (282.0, 304.0),  # Deep sculpted inner notch
    (406.0, 246.0),  # Top-right wing
]

# Slightly bolder geometry for 16px / 24px titlebar / tray icons
POINTER_PTS_SMALL: List[Tuple[float, float]] = [
    (170.0, 164.0),
    (216.0, 428.0),
    (278.0, 308.0),
    (418.0, 240.0),
]


def _scale_pt(pt: Tuple[float, float], scale: float) -> Tuple[float, float]:
    return (pt[0] * scale, pt[1] * scale)


def generate_svg(path: str) -> None:
    """Write a crisp, Apple-style minimalist SVG of the ClickForge icon."""
    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" fill="none">
  <defs>
    <linearGradient id="bgGrad" x1="256" y1="24" x2="256" y2="488" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="#1C1C26"/>
      <stop offset="100%" stop-color="#0B0B10"/>
    </linearGradient>
    <linearGradient id="borderGrad" x1="256" y1="24" x2="256" y2="488" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="#3A3A50"/>
      <stop offset="100%" stop-color="#1D1D28"/>
    </linearGradient>
    <radialGradient id="ambientGlow" cx="186" cy="180" r="150" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="{ACCENT_HEX}" stop-opacity="0.28"/>
      <stop offset="100%" stop-color="{ACCENT_HEX}" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="ringGrad" x1="120" y1="114" x2="252" y2="246" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="{ACCENT_LIGHT_HEX}"/>
      <stop offset="100%" stop-color="{ACCENT_HEX}"/>
    </linearGradient>
    <filter id="ptrShadow" x="-20%" y="-20%" width="150%" height="150%">
      <feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000000" flood-opacity="0.6"/>
    </filter>
  </defs>

  <!-- Apple-Style Squircle Surface -->
  <rect x="24" y="24" width="464" height="464" rx="112" fill="url(#bgGrad)" stroke="url(#borderGrad)" stroke-width="5"/>

  <!-- Soft Ambient Radial Glow -->
  <circle cx="186" cy="180" r="150" fill="url(#ambientGlow)"/>

  <!-- Concentric Precision Touch Rings -->
  <circle cx="186" cy="180" r="102" stroke="{ACCENT_HEX}" stroke-width="9" stroke-opacity="0.36"/>
  <circle cx="186" cy="180" r="58" stroke="url(#ringGrad)" stroke-width="14"/>

  <!-- Minimalist Sculpted Chevron Pointer -->
  <path d="M 186 180 L 228 414 L 282 304 L 406 246 Z"
        fill="{WHITE_HEX}"
        stroke="{WHITE_HEX}"
        stroke-width="16"
        stroke-linejoin="round"
        filter="url(#ptrShadow)"/>
</svg>
"""
    with open(path, "w", encoding="utf-8") as f:
        f.write(svg)


def _draw_rounded_polygon(
    draw: ImageDraw.ImageDraw,
    pts: List[Tuple[float, float]],
    radius: float,
    fill: Tuple[int, int, int, int],
) -> None:
    """Draw a polygon with smoothly rounded vertices for an Apple-like finish."""
    draw.polygon(pts, fill=fill)
    if radius >= 1.0:
        w = max(1, int(round(radius * 2)))
        n = len(pts)
        for i in range(n):
            p0 = pts[i]
            p1 = pts[(i + 1) % n]
            draw.line([p0, p1], fill=fill, width=w)
        for x, y in pts:
            draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=fill)


def render_icon(size: int) -> Image.Image:
    """Render the minimalist icon at ``size`` using 4x supersampling.

    All translucent elements are composited via ``Image.alpha_composite`` so
    the squircle canvas always maintains full opacity inside its mask.
    """
    ss = 4 if size < 256 else 2
    canvas_px = size * ss
    scale = canvas_px / 512.0
    small = size <= 24

    # 1. Vertical gradient squircle background
    pad = int(round((14 if small else 24) * scale))
    radius = int(round((116 if small else 112) * scale))
    border_w = max(1, int(round((8 if small else 5) * scale)))

    grad = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    grad_draw = ImageDraw.Draw(grad)
    y0, y1 = pad, canvas_px - pad - 1
    span = max(1, y1 - y0)
    for y in range(y0, y1 + 1):
        t = (y - y0) / span
        r = int(round(BG_TOP_RGB[0] + (BG_BOT_RGB[0] - BG_TOP_RGB[0]) * t))
        g = int(round(BG_TOP_RGB[1] + (BG_BOT_RGB[1] - BG_TOP_RGB[1]) * t))
        b = int(round(BG_TOP_RGB[2] + (BG_BOT_RGB[2] - BG_TOP_RGB[2]) * t))
        grad_draw.line([(pad, y), (canvas_px - pad - 1, y)], fill=(r, g, b, 255))

    mask = Image.new("L", (canvas_px, canvas_px), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle(
        [pad, pad, canvas_px - pad - 1, canvas_px - pad - 1],
        radius=radius,
        fill=255,
    )

    img = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    img.paste(grad, (0, 0), mask)

    # 2. Ambient glow + Concentric precision touch rings (alpha-composited)
    ox, oy = _scale_pt((170.0, 164.0) if small else ORIGIN, scale)

    if not small:
        glow_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
        glow_draw = ImageDraw.Draw(glow_layer)
        glow_r = 115.0 * scale
        glow_draw.ellipse(
            [ox - glow_r, oy - glow_r, ox + glow_r, oy + glow_r],
            fill=AMBIENT_GLOW_RGBA,
        )
        glow_layer = glow_layer.filter(
            ImageFilter.GaussianBlur(radius=max(1, int(round(28.0 * scale))))
        )
        img = Image.alpha_composite(img, glow_layer)

        outer_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
        outer_draw = ImageDraw.Draw(outer_layer)
        outer_r = 102.0 * scale
        outer_w = max(2, int(round(9.0 * scale)))
        outer_draw.ellipse(
            [ox - outer_r, oy - outer_r, ox + outer_r, oy + outer_r],
            outline=OUTER_RING_RGBA,
            width=outer_w,
        )
        img = Image.alpha_composite(img, outer_layer)

    inner_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    inner_draw = ImageDraw.Draw(inner_layer)
    inner_r = (66.0 if small else 58.0) * scale
    inner_w = max(2, int(round((22.0 if small else 14.0) * scale)))
    inner_draw.ellipse(
        [ox - inner_r, oy - inner_r, ox + inner_r, oy + inner_r],
        outline=INNER_RING_RGBA,
        width=inner_w,
    )
    img = Image.alpha_composite(img, inner_layer)

    # 3. Sculpted minimalist chevron pointer with soft shadow
    raw_pts = POINTER_PTS_SMALL if small else POINTER_PTS
    pts = [_scale_pt(p, scale) for p in raw_pts]
    join_r = (6.0 if small else 8.0) * scale

    if not small:
        sh_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
        sh_draw = ImageDraw.Draw(sh_layer)
        sh_off_y = 10.0 * scale
        sh_pts = [(x, y + sh_off_y) for x, y in pts]
        _draw_rounded_polygon(sh_draw, sh_pts, join_r, SHADOW_RGBA)
        blur_px = max(1, int(round(11.0 * scale)))
        sh_layer = sh_layer.filter(ImageFilter.GaussianBlur(radius=blur_px))
        img = Image.alpha_composite(img, sh_layer)

    ptr_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    ptr_draw = ImageDraw.Draw(ptr_layer)
    _draw_rounded_polygon(ptr_draw, pts, join_r, POINTER_RGBA)
    img = Image.alpha_composite(img, ptr_layer)

    # 4. Crisp squircle border on top
    border_layer = Image.new("RGBA", (canvas_px, canvas_px), (0, 0, 0, 0))
    border_draw = ImageDraw.Draw(border_layer)
    border_draw.rounded_rectangle(
        [pad, pad, canvas_px - pad - 1, canvas_px - pad - 1],
        radius=radius,
        outline=BORDER_RGBA,
        width=border_w,
    )
    img = Image.alpha_composite(img, border_layer)

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
    frames[-1].save(
        ico_path,
        format="ICO",
        append_images=frames[:-1],
        sizes=[(s, s) for s in ICO_SIZES],
    )

    icns_path = os.path.join(ASSETS_DIR, "icon.icns")
    render_icon(1024).save(icns_path, format="ICNS")

    print(f"Generated: {svg_path}")
    print(f"Generated: {png_path} (512x512)")
    print(f"Generated: {ico_path} (frames: {ICO_SIZES})")
    print(f"Generated: {icns_path} (1024x1024 master)")


if __name__ == "__main__":
    main()
