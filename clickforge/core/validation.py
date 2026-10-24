"""Input parsing and sanitizing helpers shared by the UI and config layers."""

from __future__ import annotations

import math
import re
from typing import Optional

from clickforge.core.clicker import MAX_INTERVAL_MS, MIN_INTERVAL_MS

MAX_CPS = 1000.0 / MIN_INTERVAL_MS
SPEED_UNITS = ("ms", "sec", "min", "cps")

# Keystroke filters: they allow partial input such as "", "-" or "1.".
_FLOAT_RE = re.compile(r"\d{0,8}(\.\d{0,3})?")
_UINT_RE = re.compile(r"\d{0,8}")
_INT_RE = re.compile(r"-?\d{0,6}")


def allow_float(text: str) -> bool:
    return _FLOAT_RE.fullmatch(text) is not None


def allow_uint(text: str) -> bool:
    return _UINT_RE.fullmatch(text) is not None


def allow_int(text: str) -> bool:
    return _INT_RE.fullmatch(text) is not None


def parse_float(text: object) -> Optional[float]:
    try:
        v = float(str(text).strip())
    except (TypeError, ValueError):
        return None
    return v if math.isfinite(v) else None


def parse_int(text: object) -> Optional[int]:
    v = parse_float(text)
    return None if v is None else int(round(v))


def clamp(v: float, lo: float, hi: float) -> float:
    return lo if v < lo else hi if v > hi else v


def to_interval_ms(value: float, unit: str) -> float:
    """Convert a speed value in ``unit`` to a safe interval in milliseconds."""
    if unit == "cps":
        value = clamp(value, 0.0001, MAX_CPS)
        ms = 1000.0 / value
    elif unit == "sec":
        ms = value * 1000.0
    elif unit == "min":
        ms = value * 60_000.0
    else:
        ms = value
    return clamp(ms, MIN_INTERVAL_MS, MAX_INTERVAL_MS)


def from_interval_ms(ms: float, unit: str) -> float:
    """Inverse of :func:`to_interval_ms`."""
    ms = clamp(ms, MIN_INTERVAL_MS, MAX_INTERVAL_MS)
    if unit == "cps":
        return 1000.0 / ms
    if unit == "sec":
        return ms / 1000.0
    if unit == "min":
        return ms / 60_000.0
    return ms


def sanitize_speed(value: object, unit: str) -> Optional[float]:
    """Clamp a speed entry so its interval lands within the safe range.

    Returns ``None`` for empty, non-numeric, zero or negative input so the
    caller can fall back to the last valid value.
    """
    v = parse_float(value)
    if v is None or v <= 0:
        return None
    v = round(from_interval_ms(to_interval_ms(v, unit), unit), 3)
    return v if v > 0 else None


def fmt_number(v: float) -> str:
    """Compact number formatting without trailing zeros."""
    if abs(v - round(v)) < 1e-9:
        return str(int(round(v)))
    return f"{v:.3f}".rstrip("0").rstrip(".")
