"""Main ClickForge desktop window.

Designed with a utilitarian, high-contrast Linear / Raycast / Fluent aesthetic:
1px neutral borders, clear surface hierarchy, sans-serif UI labels paired with
monospace numeric inputs and telemetry, and zero decorative clutter.
"""

from __future__ import annotations

import os
import queue
import tkinter as tk
from tkinter import messagebox
from typing import Any, Callable, Dict, List, Optional, Tuple

import customtkinter as ctk
from PIL import Image, ImageTk

from clickforge import __app_name__, __version__
from clickforge.core.clicker import ClickEngine, ClickSettings
from clickforge.core.config import PROFILE_NAMES, Config
from clickforge.core.hotkeys import TriggerListener, display_name
from clickforge.core.system import prepare_process, resource_path, startup_warnings
from clickforge.core.validation import (
    allow_float,
    allow_int,
    allow_uint,
    clamp,
    fmt_number,
    from_interval_ms,
    parse_float,
    parse_int,
    sanitize_speed,
    to_interval_ms,
)
from clickforge.ui import theme as t
from clickforge.ui.picker import CoordinatePicker
from clickforge.ui.theme import (
    GLYPH_PIN,
    GLYPH_PINNED,
    GLYPH_THEME,
    resolve_fonts,
)
from clickforge.ui.widgets import (
    Card,
    FormRow,
    NumberEntry,
    Segmented,
    StatusBadge,
    Typography,
    accent_button,
    caption,
    ghost_button,
    icon_button,
    segmented_options,
    set_accent_enabled,
)

POLL_INTERVAL_MS = 55  # ~18 Hz UI telemetry refresh


class ClickForgeWindow(ctk.CTk):
    """Primary application window."""

    def __init__(self, config: Optional[Config] = None) -> None:
        prepare_process()
        self._config = config or Config()

        self._theme_mode: str = self._config.get("theme")
        ctk.set_appearance_mode(self._theme_mode)
        ctk.set_default_color_theme("blue")

        super().__init__()

        self.title(__app_name__)
        self.geometry("460x690")
        self.minsize(460, 690)
        self.resizable(False, False)
        self.configure(fg_color=t.BG)

        self._fonts = resolve_fonts(self)
        self._typo = Typography(self._fonts)

        # Keep icon references alive so GC never collects them
        self._icon_photos: List[ImageTk.PhotoImage] = []
        self._header_logo: Optional[ctk.CTkImage] = None
        self._apply_window_icon()
        # CustomTkinter schedules its own iconbitmap override at 200ms after __init__
        self.after(240, self._apply_window_icon)

        # Thread-safe event queue for worker & global hotkey threads
        self._events: "queue.SimpleQueue[Tuple[str, Any]]" = queue.SimpleQueue()

        self._engine = ClickEngine(
            on_stopped=lambda reason: self._events.put(("stopped", reason))
        )
        self._hotkeys = TriggerListener(
            binding=self._config.get("hotkey"),
            on_press=lambda: self._events.put(("trigger_press", None)),
            on_release=lambda: self._events.put(("trigger_release", None)),
            on_panic=lambda: self._events.put(("panic", None)),
            on_recorded=lambda key: self._events.put(("recorded", key)),
        )

        self._always_on_top: bool = bool(self._config.get("always_on_top"))
        self._trigger_mode: str = self._config.get("trigger_mode")
        self._current_profile: str = self._config.get("profile")
        self._speed_unit: str = "ms"
        self._last_valid_speed: float = 100.0
        self._recording_hotkey: bool = False
        self._picking_coords: bool = False
        self._focused_entries: int = 0
        self._loading_ui: bool = False

        self._build_ui()
        self._load_from_config()
        self._apply_always_on_top()

        # Clicking neutral background defocuses active entry fields
        self.bind_all("<Button-1>", self._on_global_click, add="+")

        self._hotkeys.start()
        self.after(POLL_INTERVAL_MS, self._poll)
        self.after(400, self._check_platform_warnings)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    # ── Window Icon & OS Chrome ──────────────────────────────────────

    def _apply_window_icon(self) -> None:
        ico_path = resource_path("assets", "icon.ico")
        png_path = resource_path("assets", "icon.png")
        try:
            if os.path.exists(ico_path):
                self.iconbitmap(ico_path)
                self.wm_iconbitmap(ico_path)
        except Exception:
            pass
        try:
            src = png_path if os.path.exists(png_path) else ico_path
            if os.path.exists(src):
                base_img = Image.open(src).convert("RGBA")
                self._icon_photos = [
                    ImageTk.PhotoImage(base_img.resize((sz, sz), Image.Resampling.LANCZOS))
                    for sz in (16, 24, 32, 48, 64, 128, 256)
                ]
                self.wm_iconphoto(True, *self._icon_photos)
        except Exception:
            pass

    def _check_platform_warnings(self) -> None:
        for msg in startup_warnings():
            messagebox.showwarning(f"{__app_name__} — Platform Notice", msg, parent=self)

    # ── UI Construction ──────────────────────────────────────────────

    def _build_ui(self) -> None:
        pad = t.PAD
        typo = self._typo

        # 1. Header Bar
        hdr = ctk.CTkFrame(self, fg_color="transparent")
        hdr.pack(fill="x", padx=pad, pady=(14, 10))

        brand = ctk.CTkFrame(hdr, fg_color="transparent")
        brand.pack(side="left")

        png_path = resource_path("assets", "icon.png")
        if os.path.exists(png_path):
            try:
                pil_logo = Image.open(png_path).convert("RGBA")
                self._header_logo = ctk.CTkImage(
                    light_image=pil_logo, dark_image=pil_logo, size=(22, 22)
                )
                ctk.CTkLabel(brand, text="", image=self._header_logo).pack(
                    side="left", padx=(0, 8)
                )
            except Exception:
                pass

        ctk.CTkLabel(
            brand, text=__app_name__, font=typo.title, text_color=t.TEXT
        ).pack(side="left")
        ctk.CTkLabel(
            brand,
            text=f"v{__version__}",
            font=typo.mono_small,
            text_color=t.TEXT_FAINT,
        ).pack(side="left", padx=(6, 0), pady=(2, 0))

        # Right-side header controls: Theme | Pin | Status Badge
        self.theme_btn = icon_button(
            hdr,
            typo,
            glyph=GLYPH_THEME.get(self._theme_mode, GLYPH_THEME["System"]),
            fallback=self._theme_mode,
            command=self._cycle_theme,
        )
        self.theme_btn.pack(side="right", padx=(6, 0))

        self.pin_btn = icon_button(
            hdr,
            typo,
            glyph=GLYPH_PIN,
            fallback="Pin",
            command=self._toggle_always_on_top,
        )
        self.pin_btn.pack(side="right", padx=(6, 0))

        self.status_badge = StatusBadge(hdr, typo)
        self.status_badge.pack(side="right")

        # 2. Telemetry & Profile Bar
        telemetry_card = Card(self, typo)
        telemetry_card.pack(fill="x", padx=pad, pady=(0, 8))

        tel_top = ctk.CTkFrame(telemetry_card.body, fg_color="transparent")
        tel_top.pack(fill="x")

        # Left: Live CPS + Total Clicks
        cps_box = ctk.CTkFrame(tel_top, fg_color="transparent")
        cps_box.pack(side="left")

        self.cps_value_lbl = ctk.CTkLabel(
            cps_box,
            text="0",
            font=typo.mono_large,
            text_color=t.TEXT,
            anchor="w",
        )
        self.cps_value_lbl.pack(side="left")

        cps_meta = ctk.CTkFrame(cps_box, fg_color="transparent")
        cps_meta.pack(side="left", padx=(8, 0))
        ctk.CTkLabel(
            cps_meta,
            text="LIVE CPS",
            font=typo.section,
            text_color=t.TEXT_FAINT,
            anchor="w",
            height=14,
        ).pack(anchor="w")
        self.total_value_lbl = ctk.CTkLabel(
            cps_meta,
            text="Total: 0",
            font=typo.mono_small,
            text_color=t.TEXT_MUTED,
            anchor="w",
            height=16,
        )
        self.total_value_lbl.pack(anchor="w")

        # Right: Reset button
        self.reset_btn = ghost_button(
            tel_top, typo, text="Reset", command=self._reset_counters, width=62
        )
        self.reset_btn.pack(side="right")

        # Preset selector bar inside telemetry card
        preset_row = ctk.CTkFrame(telemetry_card.body, fg_color="transparent")
        preset_row.pack(fill="x", pady=(10, 0))
        ctk.CTkLabel(
            preset_row,
            text="Preset",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
            width=52,
            anchor="w",
        ).pack(side="left")
        self.preset_seg = Segmented(
            preset_row,
            typo,
            options=segmented_options(PROFILE_NAMES, PROFILE_NAMES),
            command=self._on_preset_selected,
        )
        self.preset_seg.pack(side="left", fill="x", expand=True)

        # 3. Speed & Repeat Card
        speed_card = Card(self, typo, title="Speed & Repeat")
        speed_card.pack(fill="x", padx=pad, pady=(0, 8))

        self.speed_row = FormRow(speed_card.body, typo, label="Interval")
        self.speed_entry = self._make_entry(
            self.speed_row.content, allow_float, width=78, on_commit=self._commit_speed
        )
        self.speed_entry.pack(side="left", padx=(0, 6))

        self.unit_seg = Segmented(
            self.speed_row.content,
            typo,
            options={"ms": "ms", "sec": "sec", "min": "min", "cps": "CPS"},
            command=self._on_unit_changed,
        )
        self.unit_seg.pack(side="left", fill="x", expand=True)

        self.speed_hint = caption(speed_card.body, typo, text="= 10.0 CPS", mono=True)
        self.speed_hint.pack(fill="x", padx=(FormRow.LABEL_WIDTH, 0), pady=(0, 6))

        self.repeat_row = FormRow(speed_card.body, typo, label="Repeat", pady=(0, 0))
        self.repeat_seg = Segmented(
            self.repeat_row.content,
            typo,
            options={"infinite": "Infinite", "fixed": "Fixed"},
            command=self._on_repeat_changed,
        )
        self.repeat_seg.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.repeat_count_entry = self._make_entry(
            self.repeat_row.content, allow_uint, width=72, on_commit=self._commit_repeat
        )
        self.repeat_count_entry.pack(side="left")
        self.repeat_unit_lbl = ctk.CTkLabel(
            self.repeat_row.content,
            text="clicks",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
        )
        self.repeat_unit_lbl.pack(side="left", padx=(6, 0))

        # 4. Action & Target Position Card
        target_card = Card(self, typo, title="Action & Target")
        target_card.pack(fill="x", padx=pad, pady=(0, 8))

        self.button_row = FormRow(target_card.body, typo, label="Button")
        self.button_seg = Segmented(
            self.button_row.content,
            typo,
            options={"left": "Left", "right": "Right", "middle": "Middle"},
            command=lambda _: self._mark_custom_and_save(),
        )
        self.button_seg.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.click_type_seg = Segmented(
            self.button_row.content,
            typo,
            options={"single": "1x", "double": "2x"},
            command=lambda _: self._mark_custom_and_save(),
            width=84,
        )
        self.click_type_seg.pack(side="left")

        self.pos_mode_row = FormRow(target_card.body, typo, label="Position")
        self.pos_seg = Segmented(
            self.pos_mode_row.content,
            typo,
            options={"cursor": "Follow Cursor", "fixed": "Fixed XY"},
            command=self._on_position_mode_changed,
        )
        self.pos_seg.pack(side="left", fill="x", expand=True)

        self.coords_row = FormRow(target_card.body, typo, label="Coords", pady=(0, 0))
        self.x_tag_lbl = ctk.CTkLabel(
            self.coords_row.content,
            text="X",
            font=typo.mono_small,
            text_color=t.TEXT_MUTED,
            width=14,
        )
        self.x_tag_lbl.pack(side="left")
        self.pos_x_entry = self._make_entry(
            self.coords_row.content, allow_int, width=72, on_commit=self._commit_coords
        )
        self.pos_x_entry.pack(side="left", padx=(2, 8))

        self.y_tag_lbl = ctk.CTkLabel(
            self.coords_row.content,
            text="Y",
            font=typo.mono_small,
            text_color=t.TEXT_MUTED,
            width=14,
        )
        self.y_tag_lbl.pack(side="left")
        self.pos_y_entry = self._make_entry(
            self.coords_row.content, allow_int, width=72, on_commit=self._commit_coords
        )
        self.pos_y_entry.pack(side="left", padx=(2, 8))

        # High-contrast Pick button (white bold text on #7C3AED violet)
        self.pick_btn = accent_button(
            self.coords_row.content,
            typo,
            text="Pick",
            command=self._start_coordinate_picker,
            width=68,
        )
        self.pick_btn.pack(side="right", fill="x", expand=True)

        # 5. Humanizer (Anti-Pattern Randomization) Card
        human_card = Card(self, typo, title="Humanizer")
        human_card.pack(fill="x", padx=pad, pady=(0, 8))

        self.humanize_var = tk.BooleanVar(value=False)
        self.humanize_switch = ctk.CTkSwitch(
            human_card.header,
            text="Enable",
            variable=self.humanize_var,
            font=typo.caption,
            text_color=t.TEXT_MUTED,
            fg_color=t.BORDER_STRONG,
            progress_color=t.ACCENT,
            button_color=t.TEXT_ON_ACCENT,
            button_hover_color=t.TEXT,
            switch_width=34,
            switch_height=18,
            command=self._on_humanize_toggled,
        )
        self.humanize_switch.pack(side="right")

        self.jitter_row = FormRow(human_card.body, typo, label="Jitter")
        self.jitter_prefix_lbl = ctk.CTkLabel(
            self.jitter_row.content,
            text="±",
            font=typo.mono_small,
            text_color=t.TEXT_MUTED,
        )
        self.jitter_prefix_lbl.pack(side="left", padx=(0, 3))
        self.jitter_entry = self._make_entry(
            self.jitter_row.content, allow_float, width=54, on_commit=self._commit_humanizer
        )
        self.jitter_entry.pack(side="left")
        self.jitter_unit_lbl = ctk.CTkLabel(
            self.jitter_row.content,
            text="% σ",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
        )
        self.jitter_unit_lbl.pack(side="left", padx=(4, 14))

        self.pos_jitter_lbl = ctk.CTkLabel(
            self.jitter_row.content,
            text="Spread ±",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
        )
        self.pos_jitter_lbl.pack(side="left", padx=(0, 4))
        self.pos_jitter_entry = self._make_entry(
            self.jitter_row.content, allow_uint, width=48, on_commit=self._commit_humanizer
        )
        self.pos_jitter_entry.pack(side="left")
        self.pos_jitter_unit_lbl = ctk.CTkLabel(
            self.jitter_row.content,
            text="px",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
        )
        self.pos_jitter_unit_lbl.pack(side="left", padx=(4, 0))

        self.hold_row = FormRow(human_card.body, typo, label="Hold Duty", pady=(0, 0))
        self.hold_min_entry = self._make_entry(
            self.hold_row.content, allow_float, width=56, on_commit=self._commit_humanizer
        )
        self.hold_min_entry.pack(side="left")
        self.hold_sep_lbl = ctk.CTkLabel(
            self.hold_row.content,
            text="to",
            font=typo.caption,
            text_color=t.TEXT_MUTED,
        )
        self.hold_sep_lbl.pack(side="left", padx=6)
        self.hold_max_entry = self._make_entry(
            self.hold_row.content, allow_float, width=56, on_commit=self._commit_humanizer
        )
        self.hold_max_entry.pack(side="left")
        self.hold_unit_lbl = ctk.CTkLabel(
            self.hold_row.content,
            text="ms  (Gaussian timing)",
            font=typo.caption,
            text_color=t.TEXT_FAINT,
        )
        self.hold_unit_lbl.pack(side="left", padx=(6, 0))

        # 6. Trigger & Primary Action Footer
        footer = ctk.CTkFrame(self, fg_color="transparent")
        footer.pack(fill="x", padx=pad, pady=(2, 12))

        trig_row = ctk.CTkFrame(footer, fg_color="transparent")
        trig_row.pack(fill="x", pady=(0, 8))

        ctk.CTkLabel(
            trig_row,
            text="Trigger",
            font=typo.label,
            text_color=t.TEXT_MUTED,
            width=54,
            anchor="w",
        ).pack(side="left")

        self.trigger_seg = Segmented(
            trig_row,
            typo,
            options={"toggle": "Toggle", "hold": "Hold"},
            command=self._on_trigger_mode_changed,
            width=136,
        )
        self.trigger_seg.pack(side="left", padx=(0, 8))

        ctk.CTkLabel(
            trig_row,
            text="Hotkey:",
            font=typo.label,
            text_color=t.TEXT_MUTED,
        ).pack(side="left", padx=(4, 6))

        self.hotkey_btn = ghost_button(
            trig_row,
            typo,
            text="[ F6 ]",
            command=self._start_hotkey_recording,
            width=120,
            font=typo.mono,
        )
        self.hotkey_btn.pack(side="right", fill="x", expand=True)

        # Primary Start / Stop Button
        self.start_btn = accent_button(
            footer,
            typo,
            text="START (F6)",
            command=self.toggle_clicking,
            height=42,
            font=typo.button,
            radius=t.RADIUS,
        )
        self.start_btn.pack(fill="x")

        self.footer_caption = caption(
            footer,
            typo,
            text="Panic stop: Esc or move pointer to top-left corner (0, 0)   •   Supports Mouse4 / Mouse5",
        )
        self.footer_caption.configure(anchor="center")
        self.footer_caption.pack(fill="x", pady=(4, 0))

    # ── Entry Helpers & Focus Suspension ─────────────────────────────

    def _make_entry(
        self,
        master,
        allow_fn: Callable[[str], bool],
        width: int,
        on_commit: Callable[[], None],
    ) -> NumberEntry:
        entry = NumberEntry(master, self._typo, allow=allow_fn, width=width)
        entry.inner.bind("<FocusIn>", lambda _e: self._on_entry_focus(True), add="+")
        entry.inner.bind(
            "<FocusOut>",
            lambda _e: (self._on_entry_focus(False), on_commit()),
            add="+",
        )
        entry.inner.bind("<Return>", lambda _e: (on_commit(), self.focus_set()), add="+")
        return entry

    def _on_entry_focus(self, gained: bool) -> None:
        self._focused_entries = max(0, self._focused_entries + (1 if gained else -1))
        self._sync_hotkey_suspension()

    def _sync_hotkey_suspension(self) -> None:
        suspend = self._focused_entries > 0 or self._picking_coords or self._recording_hotkey
        self._hotkeys.set_suspended(suspend)

    def _on_global_click(self, event: tk.Event) -> None:
        widget = event.widget
        if not isinstance(widget, tk.Entry):
            try:
                self.focus_set()
            except Exception:
                pass

    # ── Interactive State & Validation ───────────────────────────────

    def _update_speed_hint(self) -> None:
        val = sanitize_speed(self.speed_entry.get_text(), self._speed_unit)
        if val is None:
            val = self._last_valid_speed
        ms = to_interval_ms(val, self._speed_unit)
        cps_est = 1000.0 / ms
        if self._speed_unit == "cps":
            self.speed_row.label.configure(text="Target CPS")
            self.speed_hint.configure(text=f"Base interval: {fmt_number(ms)} ms")
        else:
            self.speed_row.label.configure(text="Interval")
            self.speed_hint.configure(
                text=f"Effective rate: ~{fmt_number(round(cps_est, 1))} CPS   (min 1 ms)"
            )

    def _commit_speed(self) -> float:
        clean = sanitize_speed(self.speed_entry.get_text(), self._speed_unit)
        if clean is None:
            self.speed_entry.flash_error()
            clean = self._last_valid_speed
        else:
            self._last_valid_speed = clean
        self.speed_entry.set_text(fmt_number(clean))
        self._update_speed_hint()
        if not self._loading_ui:
            self._mark_custom_and_save()
        return clean

    def _commit_repeat(self) -> int:
        val = parse_int(self.repeat_count_entry.get_text())
        if val is None or val < 1:
            self.repeat_count_entry.flash_error()
            val = max(1, int(self._config.get("settings").get("repeat_count", 10)))
        val = min(val, 10_000_000)
        self.repeat_count_entry.set_text(str(val))
        if not self._loading_ui:
            self._mark_custom_and_save()
        return val

    def _commit_coords(self) -> Tuple[int, int]:
        vx = parse_int(self.pos_x_entry.get_text())
        vy = parse_int(self.pos_y_entry.get_text())
        if vx is None:
            self.pos_x_entry.flash_error()
            vx = int(self._config.get("settings").get("fixed_x", 0))
        if vy is None:
            self.pos_y_entry.flash_error()
            vy = int(self._config.get("settings").get("fixed_y", 0))
        vx = int(clamp(vx, -32768, 32767))
        vy = int(clamp(vy, -32768, 32767))
        self.pos_x_entry.set_text(str(vx))
        self.pos_y_entry.set_text(str(vy))
        if not self._loading_ui:
            self._mark_custom_and_save()
        return vx, vy

    def _commit_humanizer(self) -> Tuple[float, float, float, int]:
        jp = parse_float(self.jitter_entry.get_text())
        if jp is None or jp < 0:
            self.jitter_entry.flash_error()
            jp = 15.0
        jp = round(clamp(jp, 0.0, 50.0), 1)

        h_min = parse_float(self.hold_min_entry.get_text())
        h_max = parse_float(self.hold_max_entry.get_text())
        if h_min is None or h_min < 0:
            self.hold_min_entry.flash_error()
            h_min = 18.0
        if h_max is None or h_max < 0:
            self.hold_max_entry.flash_error()
            h_max = 42.0
        h_min = round(clamp(h_min, 0.0, 500.0), 1)
        h_max = round(clamp(h_max, 0.0, 500.0), 1)
        if h_min > h_max:
            h_min, h_max = h_max, h_min

        pj = parse_int(self.pos_jitter_entry.get_text())
        if pj is None or pj < 0:
            self.pos_jitter_entry.flash_error()
            pj = 2
        pj = int(clamp(pj, 0, 50))

        self.jitter_entry.set_text(fmt_number(jp))
        self.hold_min_entry.set_text(fmt_number(h_min))
        self.hold_max_entry.set_text(fmt_number(h_max))
        self.pos_jitter_entry.set_text(str(pj))
        if not self._loading_ui:
            self._mark_custom_and_save()
        return jp, h_min, h_max, pj

    def _refresh_interactive_states(self) -> None:
        running = self._engine.is_running
        editable = not running

        # Top preset & speed controls
        self.preset_seg.set_enabled(editable)
        self.speed_row.set_enabled(editable)
        self.speed_entry.set_enabled(editable)
        self.unit_seg.set_enabled(editable)

        # Repeat controls
        self.repeat_row.set_enabled(editable)
        self.repeat_seg.set_enabled(editable)
        fixed_repeat = editable and (self.repeat_seg.get_key() == "fixed")
        self.repeat_count_entry.set_enabled(fixed_repeat)
        self.repeat_unit_lbl.configure(
            text_color=t.TEXT_MUTED if fixed_repeat else t.TEXT_FAINT
        )

        # Action & Position controls
        self.button_row.set_enabled(editable)
        self.button_seg.set_enabled(editable)
        self.click_type_seg.set_enabled(editable)
        self.pos_mode_row.set_enabled(editable)
        self.pos_seg.set_enabled(editable)

        fixed_pos = editable and (self.pos_seg.get_key() == "fixed")
        self.coords_row.set_enabled(fixed_pos)
        self.x_tag_lbl.configure(text_color=t.TEXT_MUTED if fixed_pos else t.TEXT_FAINT)
        self.y_tag_lbl.configure(text_color=t.TEXT_MUTED if fixed_pos else t.TEXT_FAINT)
        self.pos_x_entry.set_enabled(fixed_pos)
        self.pos_y_entry.set_enabled(fixed_pos)
        set_accent_enabled(self.pick_btn, fixed_pos)

        # Humanizer controls
        self.humanize_switch.configure(state="normal" if editable else "disabled")
        hum_active = editable and bool(self.humanize_var.get())
        self.jitter_row.set_enabled(hum_active)
        self.hold_row.set_enabled(hum_active)
        self.jitter_entry.set_enabled(hum_active)
        self.hold_min_entry.set_enabled(hum_active)
        self.hold_max_entry.set_enabled(hum_active)

        pos_jit_active = hum_active and (self.pos_seg.get_key() == "fixed")
        self.pos_jitter_entry.set_enabled(pos_jit_active)
        self.pos_jitter_lbl.configure(
            text_color=t.TEXT_MUTED if pos_jit_active else t.TEXT_FAINT
        )
        self.pos_jitter_unit_lbl.configure(
            text_color=t.TEXT_MUTED if pos_jit_active else t.TEXT_FAINT
        )

        # Trigger & Hotkey controls
        self.trigger_seg.set_enabled(editable)
        self.hotkey_btn.configure(state="normal" if editable else "disabled")

        # Status badge & primary Start/Stop button
        self.status_badge.set_running(running)
        hk_label = display_name(self._hotkeys.binding)
        if running:
            self.start_btn.configure(
                text=f"STOP ({hk_label})",
                fg_color=t.DANGER,
                hover_color=t.DANGER_HOVER,
            )
        else:
            self.start_btn.configure(
                text=f"START ({hk_label})",
                fg_color=t.ACCENT,
                hover_color=t.ACCENT_HOVER,
            )
        if not self._recording_hotkey:
            self.hotkey_btn.configure(text=f"[ {hk_label} ]")

    # ── Widget Event Handlers ────────────────────────────────────────

    def _on_unit_changed(self, new_unit: str) -> None:
        if new_unit == self._speed_unit:
            return
        if self.unit_seg.get_key() != new_unit:
            self.unit_seg.set_key(new_unit)
        cur_val = sanitize_speed(self.speed_entry.get_text(), self._speed_unit)
        if cur_val is None:
            cur_val = self._last_valid_speed
        ms = to_interval_ms(cur_val, self._speed_unit)
        self._speed_unit = new_unit
        converted = round(from_interval_ms(ms, new_unit), 3)
        self._last_valid_speed = converted
        self.speed_entry.set_text(fmt_number(converted))
        self._update_speed_hint()
        if not self._loading_ui:
            self._mark_custom_and_save()

    def _on_repeat_changed(self, mode: str) -> None:
        if self.repeat_seg.get_key() != mode:
            self.repeat_seg.set_key(mode)
        self._refresh_interactive_states()
        if not self._loading_ui:
            self._mark_custom_and_save()

    def _on_position_mode_changed(self, mode: str) -> None:
        if self.pos_seg.get_key() != mode:
            self.pos_seg.set_key(mode)
        self._refresh_interactive_states()
        if not self._loading_ui:
            self._mark_custom_and_save()

    def _on_humanize_toggled(self) -> None:
        self._refresh_interactive_states()
        if not self._loading_ui:
            self._mark_custom_and_save()

    def _on_trigger_mode_changed(self, mode: str) -> None:
        self._trigger_mode = mode
        self._config.set("trigger_mode", mode)
        self._config.save()

    def _on_preset_selected(self, preset_name: str) -> None:
        if self._loading_ui or self._engine.is_running:
            return
        self._current_profile = preset_name
        self._config.set("profile", preset_name)
        profile_data = self._config.preset(preset_name)
        self._config.set("settings", profile_data)
        self._config.save()
        self._apply_profile_dict(profile_data)

    def _cycle_theme(self) -> None:
        order = ("System", "Dark", "Light")
        idx = order.index(self._theme_mode) if self._theme_mode in order else 0
        self._theme_mode = order[(idx + 1) % len(order)]
        ctk.set_appearance_mode(self._theme_mode)
        self._config.set("theme", self._theme_mode)
        self._config.save()
        self._update_header_buttons()
        # Re-assert custom window icon after CTk's titlebar color reset
        self.after(150, self._apply_window_icon)

    def _toggle_always_on_top(self) -> None:
        self._always_on_top = not self._always_on_top
        self._apply_always_on_top()
        self._config.set("always_on_top", self._always_on_top)
        self._config.save()

    def _apply_always_on_top(self) -> None:
        try:
            self.attributes("-topmost", self._always_on_top)
        except tk.TclError:
            pass
        self._update_header_buttons()

    def _update_header_buttons(self) -> None:
        has_glyphs = self._typo.fonts.icons is not None
        if has_glyphs:
            self.theme_btn.configure(
                text=GLYPH_THEME.get(self._theme_mode, GLYPH_THEME["System"])
            )
            self.pin_btn.configure(
                text=GLYPH_PINNED if self._always_on_top else GLYPH_PIN,
                text_color=t.ACCENT if self._always_on_top else t.TEXT_MUTED,
                border_color=t.ACCENT if self._always_on_top else t.BORDER,
            )
        else:
            self.theme_btn.configure(text=self._theme_mode)
            self.pin_btn.configure(
                text="Pinned" if self._always_on_top else "Pin",
                text_color=t.ACCENT if self._always_on_top else t.TEXT_MUTED,
                border_color=t.ACCENT if self._always_on_top else t.BORDER,
            )

    def _reset_counters(self) -> None:
        self._engine.reset_counter()
        self.cps_value_lbl.configure(text="0")
        self.total_value_lbl.configure(text="Total: 0")

    # ── Coordinate Picker & Hotkey Recording ─────────────────────────

    def _start_coordinate_picker(self) -> None:
        if self._picking_coords or self._engine.is_running:
            return
        self._picking_coords = True
        self._sync_hotkey_suspension()
        self.pick_btn.configure(text="Picking...", fg_color=t.WARNING)
        CoordinatePicker(self, self._fonts, on_done=self._on_coordinate_picked).start()

    def _on_coordinate_picked(self, coords: Optional[Tuple[int, int]]) -> None:
        self._picking_coords = False
        self._sync_hotkey_suspension()
        self.pick_btn.configure(text="Pick", fg_color=t.ACCENT)
        if coords is not None:
            x, y = coords
            self.pos_x_entry.set_text(str(x))
            self.pos_y_entry.set_text(str(y))
            self._mark_custom_and_save()

    def _start_hotkey_recording(self) -> None:
        if self._recording_hotkey or self._engine.is_running:
            return
        self._recording_hotkey = True
        self._sync_hotkey_suspension()
        self.hotkey_btn.configure(
            text="Press key / Mouse4-5...",
            border_color=t.WARNING,
            text_color=t.WARNING,
        )
        self._hotkeys.begin_recording()

    def _finish_hotkey_recording(self, binding: Optional[str]) -> None:
        self._recording_hotkey = False
        self._sync_hotkey_suspension()
        if binding:
            self._hotkeys.set_binding(binding)
            self._config.set("hotkey", binding)
            self._config.save()
        self.hotkey_btn.configure(border_color=t.BORDER_STRONG, text_color=t.TEXT)
        self._refresh_interactive_states()

    # ── Engine Control & Polling ─────────────────────────────────────

    def _build_settings_snapshot(self) -> ClickSettings:
        speed_val = self._commit_speed()
        interval_ms = to_interval_ms(speed_val, self._speed_unit)
        repeat_count = self._commit_repeat() if self.repeat_seg.get_key() == "fixed" else 0
        fx, fy = self._commit_coords()
        fixed_pos = (fx, fy) if self.pos_seg.get_key() == "fixed" else None
        jp, h_min, h_max, pj = self._commit_humanizer()
        return ClickSettings(
            interval_ms=interval_ms,
            button=self.button_seg.get_key() or "left",
            double=(self.click_type_seg.get_key() == "double"),
            repeat_count=repeat_count,
            fixed_position=fixed_pos,
            humanize=bool(self.humanize_var.get()),
            jitter_pct=jp,
            hold_min_ms=h_min,
            hold_max_ms=h_max,
            position_jitter_px=pj if fixed_pos is not None else 0,
            corner_failsafe=True,
        ).sanitized()

    def toggle_clicking(self) -> None:
        if self._engine.is_running:
            self._stop_clicking()
        else:
            self._start_clicking()

    def _start_clicking(self) -> None:
        if self._engine.is_running or self._recording_hotkey or self._picking_coords:
            return
        snapshot = self._build_settings_snapshot()
        self._save_current_settings()
        if self._engine.start(snapshot):
            self._refresh_interactive_states()

    def _stop_clicking(self) -> None:
        self._engine.stop(wait=False)
        self._refresh_interactive_states()

    def _poll(self) -> None:
        # 1. Drain worker / hotkey events queued from background threads
        while True:
            try:
                kind, payload = self._events.get_nowait()
            except queue.Empty:
                break
            if kind == "trigger_press":
                if self._trigger_mode == "hold":
                    self._start_clicking()
                else:
                    self.toggle_clicking()
            elif kind == "trigger_release":
                if self._trigger_mode == "hold" and self._engine.is_running:
                    self._stop_clicking()
            elif kind == "panic":
                if self._engine.is_running:
                    self._stop_clicking()
            elif kind == "recorded":
                self._finish_hotkey_recording(payload)
            elif kind == "stopped":
                self._refresh_interactive_states()

        # 2. Update live telemetry (rolling 1-second CPS + Total Clicks)
        cps = self._engine.cps()
        total = self._engine.click_count
        self.cps_value_lbl.configure(
            text=str(cps),
            text_color=t.ACCENT if self._engine.is_running and cps > 0 else t.TEXT,
        )
        self.total_value_lbl.configure(text=f"Total: {total:,}")

        self.after(POLL_INTERVAL_MS, self._poll)

    # ── Config Persistence & Presets ─────────────────────────────────

    def _apply_profile_dict(self, data: Dict[str, Any]) -> None:
        self._loading_ui = True
        try:
            self._speed_unit = data.get("speed_unit", "ms")
            self.unit_seg.set_key(self._speed_unit)
            val = sanitize_speed(data.get("speed_value", 100.0), self._speed_unit) or 100.0
            self._last_valid_speed = val
            self.speed_entry.set_text(fmt_number(val))
            self._update_speed_hint()

            self.repeat_seg.set_key(data.get("repeat_mode", "infinite"))
            self.repeat_count_entry.set_text(str(data.get("repeat_count", 10)))

            self.button_seg.set_key(data.get("button", "left"))
            self.click_type_seg.set_key(data.get("click_type", "single"))

            self.pos_seg.set_key(data.get("position_mode", "cursor"))
            self.pos_x_entry.set_text(str(data.get("fixed_x", 0)))
            self.pos_y_entry.set_text(str(data.get("fixed_y", 0)))

            self.humanize_var.set(bool(data.get("humanize", False)))
            self.jitter_entry.set_text(fmt_number(float(data.get("jitter_pct", 15.0))))
            self.hold_min_entry.set_text(fmt_number(float(data.get("hold_min_ms", 18.0))))
            self.hold_max_entry.set_text(fmt_number(float(data.get("hold_max_ms", 42.0))))
            self.pos_jitter_entry.set_text(str(int(data.get("pos_jitter_px", 2))))
        finally:
            self._loading_ui = False
        self._refresh_interactive_states()

    def _load_from_config(self) -> None:
        self._loading_ui = True
        try:
            self._current_profile = self._config.get("profile")
            if self._current_profile not in PROFILE_NAMES:
                self._current_profile = "Default"
            self.preset_seg.set_key(self._current_profile)
            self._trigger_mode = self._config.get("trigger_mode")
            self.trigger_seg.set_key(self._trigger_mode)
            self._hotkeys.set_binding(self._config.get("hotkey"))
        finally:
            self._loading_ui = False
        self._apply_profile_dict(self._config.get("settings"))

    def _collect_profile_dict(self) -> Dict[str, Any]:
        speed_val = sanitize_speed(self.speed_entry.get_text(), self._speed_unit)
        if speed_val is None:
            speed_val = self._last_valid_speed
        rc = parse_int(self.repeat_count_entry.get_text()) or 10
        fx = parse_int(self.pos_x_entry.get_text()) or 0
        fy = parse_int(self.pos_y_entry.get_text()) or 0
        jp = parse_float(self.jitter_entry.get_text())
        h_min = parse_float(self.hold_min_entry.get_text())
        h_max = parse_float(self.hold_max_entry.get_text())
        pj = parse_int(self.pos_jitter_entry.get_text())
        return {
            "speed_value": speed_val,
            "speed_unit": self._speed_unit,
            "repeat_mode": self.repeat_seg.get_key() or "infinite",
            "repeat_count": max(1, rc),
            "button": self.button_seg.get_key() or "left",
            "click_type": self.click_type_seg.get_key() or "single",
            "position_mode": self.pos_seg.get_key() or "cursor",
            "fixed_x": fx,
            "fixed_y": fy,
            "humanize": bool(self.humanize_var.get()),
            "jitter_pct": 15.0 if jp is None else clamp(jp, 0.0, 50.0),
            "hold_min_ms": 18.0 if h_min is None else clamp(h_min, 0.0, 500.0),
            "hold_max_ms": 42.0 if h_max is None else clamp(h_max, 0.0, 500.0),
            "pos_jitter_px": 2 if pj is None else int(clamp(pj, 0, 50)),
        }

    def _mark_custom_and_save(self) -> None:
        if self._loading_ui:
            return
        profile = self._collect_profile_dict()
        self._current_profile = "Custom"
        self.preset_seg.set_key("Custom")
        self._config.set("profile", "Custom")
        self._config.set("settings", profile)
        self._config.set("custom", profile)
        self._config.save()

    def _save_current_settings(self) -> None:
        profile = self._collect_profile_dict()
        self._config.set("settings", profile)
        if self._current_profile == "Custom":
            self._config.set("custom", profile)
        self._config.set("hotkey", self._hotkeys.binding)
        self._config.set("trigger_mode", self._trigger_mode)
        self._config.set("always_on_top", self._always_on_top)
        self._config.set("theme", self._theme_mode)
        self._config.save()

    def _on_close(self) -> None:
        try:
            self._save_current_settings()
        except Exception:
            pass
        self._hotkeys.stop()
        self._engine.stop(wait=True, timeout=0.8)
        self.destroy()
