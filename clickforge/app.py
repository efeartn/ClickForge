"""
ClickForge — Premium Bento Grid UI
Lora serif for labels, Segoe UI bold for actions.
Purple/blue gradient palette. Fixed styling and sizes.
"""

import tkinter as tk
from typing import Optional, Any
import customtkinter as ctk
from pynput import mouse, keyboard
import os
import sys

from clickforge import __version__, __app_name__
from clickforge.config import Config
from clickforge.clicker import ClickEngine
from clickforge.hotkey import HotkeyManager
from clickforge.platform_utils import get_platform, show_platform_warnings

# ── Colors ───────────────────────────────────────────────────────────
BG_DARK       = "#0F0E17"
BG_LIGHT      = "#F0EFFA"

CARD_A_DARK   = "#1A1730"
CARD_B_DARK   = "#1C1832"
CARD_C_DARK   = "#181A33"
CARD_D_DARK   = "#1E1934"

CARD_A_LIGHT  = "#EDE9FA"
CARD_B_LIGHT  = "#EBE8F8"
CARD_C_LIGHT  = "#E8EAF9"
CARD_D_LIGHT  = "#EDEBFA"

ACCENT        = "#7C3AED"
ACCENT_HOVER  = "#6D28D9"
ACCENT_SOFT   = "#8B5CF6"
INDIGO        = "#6366F1"
RED           = "#EF4444"
RED_HOVER     = "#DC2626"
GREEN         = "#10B981"
AMBER         = "#F59E0B"
DIM           = "#6B7280"
MUTED         = "#4B5563"

# Segmented button backgrounds (to make them borderless/seamless)
SEG_BG_DARK   = "#252140"
SEG_BG_LIGHT  = "#DFDBF0"

FONT_BODY     = "Lora"
FONT_ACTION   = "Segoe UI"
FALLBACK      = "Segoe UI"


def get_resource_path(relative_path: str) -> str:
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


def load_custom_font():
    path = get_resource_path(os.path.join("assets", "fonts", "Lora[wght].ttf"))
    if not os.path.exists(path):
        return False
    if sys.platform.startswith('win'):
        import ctypes
        return ctypes.windll.gdi32.AddFontResourceExW(path, 0x10, 0) > 0
    return False


class ClickForgeApp(ctk.CTk):
    def __init__(self):
        super().__init__()

        self._font_ok = load_custom_font()
        if not self._font_ok:
            global FONT_BODY
            FONT_BODY = FALLBACK

        self._config = Config()
        self._current_theme = self._config.get("appearance_mode", "System")
        ctk.set_appearance_mode(self._current_theme)
        ctk.set_default_color_theme("blue")

        # Force Windows to treat this as a unique app to break icon cache
        if sys.platform.startswith('win'):
            import ctypes
            myappid = f'clickforge.app.v{__version__}'
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)

        self.title(__app_name__)
        # Slightly wider and taller to fit bigger text
        self.geometry("480x680")
        self.resizable(False, False)
        self.configure(fg_color=(BG_LIGHT, BG_DARK))

        try:
            from PIL import Image, ImageTk
            ico = get_resource_path(os.path.join("assets", "icon.ico"))
            if os.path.exists(ico):
                img = Image.open(ico)
                photo = ImageTk.PhotoImage(img)
                self.iconphoto(False, photo)
        except Exception:
            pass

        self.click_engine = ClickEngine(
            on_click=lambda c: None,
            on_stopped=self._on_engine_stopped,
        )
        self.hotkey_manager = HotkeyManager(
            hotkey=self._config.get("hotkey", "f6"),
            on_toggle=self.toggle_clicking,
        )

        self.is_running = False
        self.picking_position = False
        self._mouse_listener: Optional[mouse.Listener] = None

        self._build_ui()
        self._load_settings()

        self.hotkey_manager.start()
        self._poll()
        self.after(500, self._check_platform)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _check_platform(self):
        show_platform_warnings(self)

    # ── Fonts ────────────────────────────────────────────────────────

    def _body(self, size=15, weight="normal"):
        return ctk.CTkFont(family=FONT_BODY, size=size, weight=weight)

    def _action(self, size=18, weight="bold"):
        return ctk.CTkFont(family=FONT_ACTION, size=size, weight=weight)

    # ── UI ───────────────────────────────────────────────────────────

    def _build_ui(self):
        p = 18

        # Header
        hdr = ctk.CTkFrame(self, fg_color="transparent", height=40)
        hdr.pack(fill="x", padx=p, pady=(p, 12))

        ctk.CTkLabel(hdr, text=__app_name__, font=self._body(26, "bold")).pack(side="left")

        # Theme selector: Cycling Button (fixes ugly OptionMenu border)
        self.theme_btn = ctk.CTkButton(
            hdr, text=f"Theme: {self._current_theme}", width=100, height=32,
            font=self._body(13), corner_radius=10,
            fg_color=(CARD_A_LIGHT, CARD_A_DARK),
            text_color=("black", "white"),
            hover_color=(CARD_B_LIGHT, CARD_C_DARK),
            command=self._cycle_theme,
        )
        self.theme_btn.pack(side="right", padx=(8, 0))

        # Hotkey pill
        self.hotkey_btn = ctk.CTkButton(
            hdr, text="F6", width=50, height=32, corner_radius=10,
            fg_color=ACCENT, hover_color=ACCENT_HOVER,
            font=self._body(14, "bold"), command=self._change_hotkey,
        )
        self.hotkey_btn.pack(side="right", padx=(8, 0))

        # Status badge
        self.status_label = ctk.CTkLabel(
            hdr, text=" STOPPED ", font=self._body(12, "bold"),
            fg_color=RED, text_color="white", corner_radius=8,
            padx=6, pady=2
        )
        self.status_label.pack(side="right")

        # ── Bento Grid ───────────────────────────────────────────────
        grid = ctk.CTkFrame(self, fg_color="transparent")
        grid.pack(fill="both", expand=True, padx=p, pady=(0, 10))
        grid.grid_columnconfigure(0, weight=2)
        grid.grid_columnconfigure(1, weight=3)
        grid.grid_rowconfigure(0, weight=1)
        grid.grid_rowconfigure(1, weight=1)
        self._grid = grid

        g = 12
        self._build_cps_card(0, 0, (0, g), (0, g))
        self._build_speed_card(0, 1, (0, 0), (0, g))
        self._build_position_card(1, 0, (0, g), (0, 0))
        self._build_action_card(1, 1, (0, 0), (0, 0))

        # ── START Button ─────────────────────────────────────────────
        self.start_btn = ctk.CTkButton(
            self, text="S T A R T", height=60, corner_radius=16,
            font=self._action(24, "bold"),
            fg_color=ACCENT, hover_color=ACCENT_HOVER,
            command=self.toggle_clicking,
        )
        self.start_btn.pack(fill="x", padx=p, pady=(6, p))

    # ── Cards ────────────────────────────────────────────────────────

    def _card(self, row, col, padx, pady, dark, light):
        c = ctk.CTkFrame(self._grid, corner_radius=18, fg_color=(light, dark))
        c.grid(row=row, column=col, padx=padx, pady=pady, sticky="nsew")
        return c

    def _build_cps_card(self, r, c, px, py):
        card = self._card(r, c, px, py, CARD_A_DARK, CARD_A_LIGHT)

        ctk.CTkLabel(card, text="LIVE", font=self._body(12), text_color=DIM).pack(pady=(24, 4))
        self.cps_val = ctk.CTkLabel(card, text="0", font=self._body(52, "bold"), text_color=ACCENT_SOFT)
        self.cps_val.pack()
        ctk.CTkLabel(card, text="CPS", font=self._body(14, "bold"), text_color=DIM).pack()

        self.total_lbl = ctk.CTkLabel(card, text="Total: 0", font=self._body(13), text_color=DIM)
        self.total_lbl.pack(pady=(12, 6))

        ctk.CTkButton(
            card, text="Reset", width=64, height=26, corner_radius=8,
            fg_color="transparent", border_width=1,
            border_color=(MUTED, DIM), text_color=(MUTED, DIM),
            hover_color=(CARD_B_LIGHT, ACCENT_HOVER),
            font=self._body(12), command=self._reset,
        ).pack(pady=(0, 20))

    def _build_speed_card(self, r, c, px, py):
        card = self._card(r, c, px, py, CARD_B_DARK, CARD_B_LIGHT)
        vcmd = (self.register(self._vnum), "%P")

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(expand=True)

        ctk.CTkLabel(inner, text="Interval", font=self._body(16, "bold")).pack(pady=(0, 10))

        row1 = ctk.CTkFrame(inner, fg_color="transparent")
        row1.pack()
        self.interval_entry = ctk.CTkEntry(
            row1, width=86, height=36, corner_radius=10,
            validate="key", validatecommand=vcmd, justify="center",
            font=self._body(15),
        )
        self.interval_entry.pack(side="left", padx=(0, 10))
        
        self.interval_unit = ctk.CTkSegmentedButton(
            row1, values=["ms", "sec"], width=100, height=36,
            font=self._body(14), corner_radius=10,
            fg_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_hover_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            selected_color=ACCENT, selected_hover_color=ACCENT_HOVER,
        )
        self.interval_unit.pack(side="left")
        
        self.humanize_var = ctk.BooleanVar(value=False)
        self.humanize_cb = ctk.CTkCheckBox(
            inner, text="Humanize (Randomize Delay)", variable=self.humanize_var,
            font=self._body(12), fg_color=ACCENT, hover_color=ACCENT_HOVER,
            corner_radius=6, checkbox_width=20, checkbox_height=20,
        )
        self.humanize_cb.pack(pady=(12, 0))

        ctk.CTkLabel(inner, text="Repeat", font=self._body(16, "bold")).pack(pady=(20, 10))

        mode_frame = ctk.CTkFrame(inner, fg_color="transparent")
        mode_frame.pack(fill="x", padx=36)
        
        self.mode_var = ctk.StringVar(value="continuous")
        
        ctk.CTkRadioButton(
            mode_frame, text="Infinite", variable=self.mode_var, value="continuous",
            fg_color=ACCENT, hover_color=ACCENT_HOVER,
            command=self._on_mode, font=self._body(14),
        ).pack(anchor="w")
        
        ctk.CTkRadioButton(
            mode_frame, text="Fixed", variable=self.mode_var, value="fixed_count",
            fg_color=ACCENT, hover_color=ACCENT_HOVER,
            command=self._on_mode, font=self._body(14),
        ).pack(anchor="w", pady=(12, 6))
        
        self.count_entry = ctk.CTkEntry(
            mode_frame, width=80, height=32, corner_radius=10,
            validate="key", validatecommand=vcmd, justify="center",
            font=self._body(14),
        )
        self.count_entry.pack(anchor="w", padx=(28, 0)) # Indented under "Fixed"
        self.count_entry.insert(0, "10")

    def _build_position_card(self, r, c, px, py):
        card = self._card(r, c, px, py, CARD_C_DARK, CARD_C_LIGHT)
        vcmd = (self.register(self._vnum), "%P")

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(expand=True)

        ctk.CTkLabel(inner, text="Position", font=self._body(16, "bold")).pack(pady=(0, 10))
        
        pos_frame = ctk.CTkFrame(inner, fg_color="transparent")
        pos_frame.pack(fill="x", padx=16)

        self.pos_var = ctk.StringVar(value="cursor")
        
        ctk.CTkRadioButton(
            pos_frame, text="Follow Cursor", variable=self.pos_var, value="cursor",
            fg_color=INDIGO, hover_color=ACCENT_HOVER,
            command=self._on_pos, font=self._body(14),
        ).pack(anchor="w")
        
        ctk.CTkRadioButton(
            pos_frame, text="Fixed XY", variable=self.pos_var, value="fixed",
            fg_color=INDIGO, hover_color=ACCENT_HOVER,
            command=self._on_pos, font=self._body(14),
        ).pack(anchor="w", pady=(12, 6))

        coord_row = ctk.CTkFrame(pos_frame, fg_color="transparent")
        coord_row.pack(anchor="w", padx=(28, 0)) # Indented under "Fixed XY"
        
        self.pos_x = ctk.CTkEntry(
            coord_row, width=46, height=32, corner_radius=10,
            placeholder_text="X", validate="key", validatecommand=vcmd,
            justify="center", font=self._body(13),
        )
        self.pos_x.pack(side="left")
        
        self.pos_y = ctk.CTkEntry(
            coord_row, width=46, height=32, corner_radius=10,
            placeholder_text="Y", validate="key", validatecommand=vcmd,
            justify="center", font=self._body(13),
        )
        self.pos_y.pack(side="left", padx=6)
        
        self.pick_btn = ctk.CTkButton(
            coord_row, text="Pick", width=46, height=32, corner_radius=10,
            fg_color=INDIGO, hover_color=ACCENT_HOVER,
            font=self._body(12, "bold"), command=self._pick,
        )
        self.pick_btn.pack(side="left")

    def _build_action_card(self, r, c, px, py):
        card = self._card(r, c, px, py, CARD_D_DARK, CARD_D_LIGHT)

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(expand=True)

        ctk.CTkLabel(inner, text="Mouse Button", font=self._body(16, "bold")).pack(pady=(0, 10))
        self.mouse_seg = ctk.CTkSegmentedButton(
            inner, values=["Left", "Right", "Middle"],
            width=220, height=36, font=self._body(14), corner_radius=10,
            fg_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_hover_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            selected_color=ACCENT, selected_hover_color=ACCENT_HOVER,
        )
        self.mouse_seg.pack()
        self.mouse_seg.set("Left")

        ctk.CTkLabel(inner, text="Click Type", font=self._body(16, "bold")).pack(pady=(22, 10))
        self.type_seg = ctk.CTkSegmentedButton(
            inner, values=["Single", "Double"],
            width=180, height=36, font=self._body(14), corner_radius=10,
            fg_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            unselected_hover_color=(SEG_BG_LIGHT, SEG_BG_DARK),
            selected_color=ACCENT_SOFT, selected_hover_color=ACCENT_HOVER,
        )
        self.type_seg.pack()
        self.type_seg.set("Single")

    # ── Theme ────────────────────────────────────────────────────────

    def _cycle_theme(self):
        themes = ["System", "Dark", "Light"]
        idx = themes.index(self._current_theme)
        next_theme = themes[(idx + 1) % len(themes)]
        
        self._current_theme = next_theme
        ctk.set_appearance_mode(next_theme)
        self._config.set("appearance_mode", next_theme)
        self._config.save()
        
        self.theme_btn.configure(text=f"Theme: {next_theme}")

    # ── Callbacks ────────────────────────────────────────────────────

    def _vnum(self, v):
        if v == "": return True
        try: float(v); return True
        except ValueError: return False

    def _on_mode(self):
        self.count_entry.configure(
            state="normal" if self.mode_var.get() == "fixed_count" else "disabled"
        )

    def _on_pos(self):
        fixed = self.pos_var.get() == "fixed"
        st = "normal" if fixed else "disabled"
        self.pos_x.configure(state=st)
        self.pos_y.configure(state=st)
        self.pick_btn.configure(state="normal" if fixed else "disabled")

    def _reset(self):
        self.click_engine.reset_counter()
        self.total_lbl.configure(text="Total: 0")
        self.cps_val.configure(text="0")

    def _pick(self):
        if self.picking_position: return
        self.picking_position = True
        self.pick_btn.configure(text="...", fg_color=AMBER)

        def go():
            def on_click(x, y, btn, pressed):
                if pressed:
                    self._mouse_listener.stop()
                    self.after(0, lambda: self._picked(x, y))
                    return False
            self._mouse_listener = mouse.Listener(on_click=on_click)
            self._mouse_listener.start()
        self.after(200, go)

    def _picked(self, x, y):
        self.picking_position = False
        self.pos_x.configure(state="normal")
        self.pos_y.configure(state="normal")
        self.pos_x.delete(0, "end"); self.pos_x.insert(0, str(int(x)))
        self.pos_y.delete(0, "end"); self.pos_y.insert(0, str(int(y)))
        self.pick_btn.configure(text="Pick", fg_color=INDIGO)

    def _change_hotkey(self):
        self.hotkey_btn.configure(text="...", fg_color=AMBER, state="disabled")
        self.after(200, lambda: self.hotkey_manager.start_recording(
            callback=lambda k: self.after(0, lambda: self._hotkey_done(k))
        ))

    def _hotkey_done(self, key_str):
        if key_str == "esc":
            hk = self.hotkey_manager.hotkey
            self.hotkey_btn.configure(text=hk.upper(), fg_color=ACCENT, state="normal")
            return
        self.hotkey_manager.set_hotkey(key_str)
        self._config.set("hotkey", key_str)
        self._config.save()
        self.hotkey_btn.configure(text=key_str.upper(), fg_color=ACCENT, state="normal")

    # ── Click Engine ─────────────────────────────────────────────────

    def toggle_clicking(self):
        self.after(0, self._toggle)

    def _toggle(self):
        if self.is_running:
            self.click_engine.stop()
            self._set_status(False)
        else:
            try: iv = float(self.interval_entry.get())
            except ValueError: iv = 100.0
            ms = float(iv * 1000) if self.interval_unit.get() == "sec" else float(iv)

            try: tc = int(self.count_entry.get())
            except ValueError: tc = 10
            try: fx, fy = int(self.pos_x.get()), int(self.pos_y.get())
            except ValueError: fx, fy = 0, 0

            self.click_engine.configure(
                interval_ms=ms,
                button_str=self.mouse_seg.get().lower(),
                click_type="double" if self.type_seg.get() == "Double" else "single",
                click_mode=self.mode_var.get(),
                target_count=tc,
                position_mode=self.pos_var.get(),
                fixed_position=(fx, fy),
                humanize=self.humanize_var.get(),
            )
            self.click_engine.start()
            self._set_status(True)
            self._save_settings()

    def _set_status(self, running):
        self.is_running = running
        if running:
            self.status_label.configure(text=" RUNNING ", fg_color=GREEN)
            self.start_btn.configure(text="S T O P", fg_color=RED, hover_color=RED_HOVER)
            self.interval_entry.configure(state="disabled")
            self.count_entry.configure(state="disabled")
            self.hotkey_btn.configure(state="disabled")
            self.humanize_cb.configure(state="disabled")
        else:
            self.status_label.configure(text=" STOPPED ", fg_color=RED)
            self.start_btn.configure(text="S T A R T", fg_color=ACCENT, hover_color=ACCENT_HOVER)
            self.interval_entry.configure(state="normal")
            self.hotkey_btn.configure(state="normal")
            self.humanize_cb.configure(state="normal")
            self._on_mode()

    def _on_engine_stopped(self):
        self.after(0, lambda: self._set_status(False))

    def _poll(self):
        self.cps_val.configure(text=str(self.click_engine.current_cps))
        self.total_lbl.configure(text=f"Total: {self.click_engine.click_count:,}")
        self.after(100, self._poll)

    # ── Persistence ──────────────────────────────────────────────────

    def _load_settings(self):
        c = self._config

        iv = c.get("interval", 100)
        self.interval_entry.delete(0, "end"); self.interval_entry.insert(0, str(iv))
        u = c.get("interval_unit", "ms")
        self.interval_unit.set(u if u in ("ms", "sec") else "ms")

        self.humanize_var.set(c.get("humanize", False))

        b = c.get("mouse_button", "left")
        self.mouse_seg.set({"left":"Left","right":"Right","middle":"Middle"}.get(b,"Left"))

        ct = c.get("click_type", "single")
        self.type_seg.set("Double" if ct == "double" else "Single")

        m = c.get("click_mode", "continuous")
        self.mode_var.set(m); self._on_mode()

        cnt = c.get("click_count", 10)
        self.count_entry.delete(0, "end"); self.count_entry.insert(0, str(cnt))

        pm = c.get("position_mode", "cursor")
        self.pos_var.set(pm); self._on_pos()

        self.pos_x.delete(0, "end"); self.pos_x.insert(0, str(c.get("fixed_x", 0)))
        self.pos_y.delete(0, "end"); self.pos_y.insert(0, str(c.get("fixed_y", 0)))

        hk = c.get("hotkey", "f6")
        self.hotkey_btn.configure(text=hk.upper())
        self.hotkey_manager.set_hotkey(hk)

        theme = c.get("appearance_mode", "System")
        self._current_theme = theme
        self.theme_btn.configure(text=f"Theme: {theme}")

    def _save_settings(self):
        c = self._config
        try: c.set("interval", float(self.interval_entry.get()))
        except ValueError: pass
        c.set("interval_unit", self.interval_unit.get())
        c.set("humanize", self.humanize_var.get())
        c.set("mouse_button", self.mouse_seg.get().lower())
        c.set("click_type", "double" if self.type_seg.get() == "Double" else "single")
        c.set("click_mode", self.mode_var.get())
        try: c.set("click_count", int(self.count_entry.get()))
        except ValueError: pass
        c.set("position_mode", self.pos_var.get())
        try:
            c.set("fixed_x", int(self.pos_x.get()))
            c.set("fixed_y", int(self.pos_y.get()))
        except ValueError: pass
        hk = self.hotkey_btn.cget("text").lower()
        if hk != "...": c.set("hotkey", hk)
        c.save()

    def _on_close(self):
        self._save_settings()
        self.click_engine.stop()
        self.hotkey_manager.stop()
        if self._mouse_listener: self._mouse_listener.stop()
        self.destroy()


def main():
    app = ClickForgeApp()
    app.mainloop()

if __name__ == "__main__":
    main()
