"""
Main GUI application for ClickForge.
Modern, minimalist redesign with UX and Hotkey improvements.
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

def get_resource_path(relative_path: str) -> str:
    """Get absolute path to resource, works for dev and for PyInstaller."""
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)

class ClickForgeApp(ctk.CTk):
    def __init__(self):
        # Appearance
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")

        super().__init__()
        self.title(f"{__app_name__} v{__version__}")
        self.geometry("400x560")
        self.resizable(False, False)
        
        # Set Icon
        try:
            icon_path = get_resource_path(os.path.join("assets", "icon.ico"))
            if os.path.exists(icon_path):
                self.iconbitmap(icon_path)
        except Exception:
            pass # Ignore if icon fails to load

        self._config = Config()
        self.click_engine = ClickEngine(
            on_click=self._on_click_callback,
            on_stopped=self._on_engine_stopped,
        )
        self.hotkey_manager = HotkeyManager(
            hotkey=self._config.get("hotkey", "f6"),
            on_toggle=self.toggle_clicking,
        )

        self.is_running: bool = False
        self.current_clicks: int = 0
        self.picking_position: bool = False
        self._mouse_listener: Optional[mouse.Listener] = None

        self._create_widgets()
        self._load_settings_to_ui()
        self.hotkey_manager.start()
        self._poll_updates()
        self.after(500, self._check_platform)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _check_platform(self):
        show_platform_warnings(self)

    def _create_widgets(self):
        # Container with minimalist padding
        self.main_container = ctk.CTkFrame(self, fg_color="transparent")
        self.main_container.pack(fill="both", expand=True, padx=20, pady=20)

        self._create_header(self.main_container)
        self._create_main_settings(self.main_container)
        self._create_position_settings(self.main_container)
        self._create_hotkey_settings(self.main_container)
        
        # Spacer
        ctk.CTkFrame(self.main_container, fg_color="transparent", height=10).pack()
        
        self._create_controls(self.main_container)
        self._create_footer(self.main_container)

    def _create_header(self, parent: ctk.CTkFrame):
        header_frame = ctk.CTkFrame(parent, fg_color="transparent")
        header_frame.pack(fill="x", pady=(0, 15))

        title_label = ctk.CTkLabel(
            header_frame,
            text=__app_name__,
            font=ctk.CTkFont(size=24, weight="bold"),
        )
        title_label.pack(side="left")

        self.status_label = ctk.CTkLabel(
            header_frame,
            text="Stopped",
            text_color="#ff5555",
            font=ctk.CTkFont(size=14, weight="bold"),
        )
        self.status_label.pack(side="right")

    def _create_main_settings(self, parent: ctk.CTkFrame):
        frame = ctk.CTkFrame(parent, corner_radius=10)
        frame.pack(fill="x", pady=8)

        vcmd = (self.register(self._validate_number_input), "%P")

        # Row 1: Interval
        row1 = ctk.CTkFrame(frame, fg_color="transparent")
        row1.pack(fill="x", padx=15, pady=(15, 10))
        ctk.CTkLabel(row1, text="Interval", font=ctk.CTkFont(weight="bold")).pack(side="left")
        
        self.interval_unit = ctk.CTkSegmentedButton(row1, values=["ms", "sec"], width=80)
        self.interval_unit.pack(side="right", padx=(10, 0))
        
        self.interval_entry = ctk.CTkEntry(row1, width=70, validate="key", validatecommand=vcmd, justify="center")
        self.interval_entry.pack(side="right")

        # Row 2: Mouse & Type
        row2 = ctk.CTkFrame(frame, fg_color="transparent")
        row2.pack(fill="x", padx=15, pady=5)
        
        self.mouse_btn_var = ctk.StringVar(value="Left")
        self.mouse_btn_menu = ctk.CTkOptionMenu(row2, variable=self.mouse_btn_var, values=["Left", "Right", "Middle"], width=100)
        self.mouse_btn_menu.pack(side="left")

        self.click_type_var = ctk.StringVar(value="Single")
        self.click_type_menu = ctk.CTkOptionMenu(row2, variable=self.click_type_var, values=["Single", "Double"], width=100)
        self.click_type_menu.pack(side="right")

        # Row 3: Mode
        row3 = ctk.CTkFrame(frame, fg_color="transparent")
        row3.pack(fill="x", padx=15, pady=(10, 15))
        
        self.click_mode_var = ctk.StringVar(value="continuous")
        self.radio_continuous = ctk.CTkRadioButton(row3, text="Continuous", variable=self.click_mode_var, value="continuous", command=self._on_mode_changed)
        self.radio_continuous.pack(side="left")
        
        self.radio_fixed = ctk.CTkRadioButton(row3, text="Times:", variable=self.click_mode_var, value="fixed_count", command=self._on_mode_changed)
        self.radio_fixed.pack(side="left", padx=(20, 10))
        
        self.count_entry = ctk.CTkEntry(row3, width=60, validate="key", validatecommand=vcmd, justify="center")
        self.count_entry.pack(side="left")
        self.count_entry.insert(0, "10")

    def _create_position_settings(self, parent: ctk.CTkFrame):
        frame = ctk.CTkFrame(parent, corner_radius=10)
        frame.pack(fill="x", pady=8)

        self.pos_mode_var = ctk.StringVar(value="cursor")
        vcmd = (self.register(self._validate_number_input), "%P")

        top_row = ctk.CTkFrame(frame, fg_color="transparent")
        top_row.pack(fill="x", padx=15, pady=(15, 5))
        
        ctk.CTkRadioButton(top_row, text="Cursor Position", variable=self.pos_mode_var, value="cursor", command=self._on_position_mode_changed).pack(side="left")

        bot_row = ctk.CTkFrame(frame, fg_color="transparent")
        bot_row.pack(fill="x", padx=15, pady=(5, 15))
        
        ctk.CTkRadioButton(bot_row, text="Fixed", variable=self.pos_mode_var, value="fixed", command=self._on_position_mode_changed).pack(side="left")

        self.pick_pos_btn = ctk.CTkButton(bot_row, text="Pick Pos", width=70, command=self._pick_position)
        self.pick_pos_btn.pack(side="right", padx=(10, 0))

        self.pos_y_entry = ctk.CTkEntry(bot_row, width=50, placeholder_text="Y", validate="key", validatecommand=vcmd, justify="center")
        self.pos_y_entry.pack(side="right", padx=(5, 0))

        self.pos_x_entry = ctk.CTkEntry(bot_row, width=50, placeholder_text="X", validate="key", validatecommand=vcmd, justify="center")
        self.pos_x_entry.pack(side="right")

    def _create_hotkey_settings(self, parent: ctk.CTkFrame):
        frame = ctk.CTkFrame(parent, corner_radius=10)
        frame.pack(fill="x", pady=8)

        inner = ctk.CTkFrame(frame, fg_color="transparent")
        inner.pack(fill="x", padx=15, pady=12)

        ctk.CTkLabel(inner, text="Start/Stop Hotkey:", font=ctk.CTkFont(weight="bold")).pack(side="left")
        
        # Merged change hotkey button and label for a cleaner UX
        self.hotkey_btn = ctk.CTkButton(inner, text="Hotkey: F6", width=120, command=self._change_hotkey, font=ctk.CTkFont(weight="bold"))
        self.hotkey_btn.pack(side="right")

    def _create_controls(self, parent: ctk.CTkFrame):
        frame = ctk.CTkFrame(parent, fg_color="transparent")
        frame.pack(fill="x", pady=10)

        # Big, bold start button
        self.start_btn = ctk.CTkButton(
            frame, text="START", height=45, corner_radius=8,
            font=ctk.CTkFont(size=16, weight="bold"),
            fg_color="#10B981", hover_color="#059669", command=self.toggle_clicking
        )
        self.start_btn.pack(fill="x")

        counter_row = ctk.CTkFrame(frame, fg_color="transparent")
        counter_row.pack(fill="x", pady=(10, 0))
        
        self.clicks_label = ctk.CTkLabel(counter_row, text="Clicks: 0", font=ctk.CTkFont(size=14, weight="bold"), text_color="gray")
        self.clicks_label.pack(side="left")
        
        reset_btn = ctk.CTkButton(counter_row, text="Reset", width=50, height=24, fg_color="transparent", border_width=1, command=self._reset_counter)
        reset_btn.pack(side="right")

    def _create_footer(self, parent: ctk.CTkFrame):
        footer_frame = ctk.CTkFrame(parent, fg_color="transparent")
        footer_frame.pack(fill="x", side="bottom")

        self.theme_selector = ctk.CTkSegmentedButton(footer_frame, values=["Dark", "Light"], command=self._change_appearance)
        self.theme_selector.pack(side="left")
        self.theme_selector.set("Dark")

        version_label = ctk.CTkLabel(footer_frame, text=f"v{__version__}", text_color="gray", font=ctk.CTkFont(size=11))
        version_label.pack(side="right")

    def _on_mode_changed(self):
        if self.click_mode_var.get() == "continuous":
            self.count_entry.configure(state="disabled")
        else:
            self.count_entry.configure(state="normal")

    def _on_position_mode_changed(self):
        if self.pos_mode_var.get() == "cursor":
            self.pos_x_entry.configure(state="disabled")
            self.pos_y_entry.configure(state="disabled")
            self.pick_pos_btn.configure(state="disabled")
        else:
            self.pos_x_entry.configure(state="normal")
            self.pos_y_entry.configure(state="normal")
            self.pick_pos_btn.configure(state="normal")

    def _validate_number_input(self, value: str) -> bool:
        if value == "": return True
        try:
            float(value)
            return True
        except ValueError:
            return False

    def _change_appearance(self, mode: str):
        ctk.set_appearance_mode(mode.lower())
        self._config.set("appearance_mode", mode.lower())
        self._config.save()

    def _reset_counter(self):
        self.current_clicks = 0
        self.click_engine.reset_counter()
        self.clicks_label.configure(text="Clicks: 0")

    def _pick_position(self):
        if self.picking_position: return
        self.picking_position = True
        self.pick_pos_btn.configure(text="Click...", fg_color="#F59E0B")

        def start_listener():
            def on_click(x, y, button, pressed):
                if pressed:
                    self._mouse_listener.stop()
                    self.after(0, lambda: self._on_position_picked(x, y))
                    return False

            self._mouse_listener = mouse.Listener(on_click=on_click)
            self._mouse_listener.start()

        self.after(200, start_listener)

    def _on_position_picked(self, x: float, y: float):
        self.picking_position = False
        self.pos_x_entry.configure(state="normal")
        self.pos_y_entry.configure(state="normal")
        self.pos_x_entry.delete(0, "end")
        self.pos_x_entry.insert(0, str(int(x)))
        self.pos_y_entry.delete(0, "end")
        self.pos_y_entry.insert(0, str(int(y)))
        self.pick_pos_btn.configure(text="Pick Pos", fg_color=["#3B8ED0", "#1F6AA5"])

    def _change_hotkey(self):
        self.hotkey_btn.configure(text="Listening (ESC cancels)", fg_color="#F59E0B", state="disabled")
        self.after(200, self._start_recording_hotkey)

    def _start_recording_hotkey(self):
        self.hotkey_manager.start_recording(
            callback=lambda key_str: self.after(0, lambda: self._on_hotkey_recorded(key_str))
        )

    def _on_hotkey_recorded(self, key_str: str):
        if key_str == "esc":
            current_hk = self.hotkey_manager.hotkey
            self.hotkey_btn.configure(text=f"Hotkey: {current_hk.upper()}", fg_color=["#3B8ED0", "#1F6AA5"], state="normal")
            return
            
        self.hotkey_manager.set_hotkey(key_str)
        self._config.set("hotkey", key_str)
        self._config.save()
        self.hotkey_btn.configure(text=f"Hotkey: {key_str.upper()}", fg_color=["#3B8ED0", "#1F6AA5"], state="normal")

    def toggle_clicking(self):
        self.after(0, self._do_toggle)

    def _do_toggle(self):
        if self.is_running:
            self.click_engine.stop()
            self._update_status(running=False)
        else:
            try:
                interval_val = float(self.interval_entry.get())
            except ValueError:
                interval_val = 100.0
                
            unit = self.interval_unit.get()
            
            interval_ms = float(interval_val * 1000) if unit == "sec" else float(interval_val)

            btn = self.mouse_btn_var.get().lower()
            c_type = "double" if self.click_type_var.get() == "Double" else "single"
            c_mode = self.click_mode_var.get()
            try:
                t_count = int(self.count_entry.get())
            except ValueError:
                t_count = 10
            p_mode = self.pos_mode_var.get()
            try:
                f_x, f_y = int(self.pos_x_entry.get()), int(self.pos_y_entry.get())
            except ValueError:
                f_x, f_y = 0, 0

            self.click_engine.configure(
                interval_ms=interval_ms, button_str=btn, click_type=c_type,
                click_mode=c_mode, target_count=t_count, position_mode=p_mode,
                fixed_position=(f_x, f_y),
            )
            self.click_engine.start()
            self._update_status(running=True)
            self._save_settings_from_ui()

    def _update_status(self, running: bool):
        self.is_running = running
        if running:
            self.status_label.configure(text="Running", text_color="#10B981")
            self.start_btn.configure(text="STOP", fg_color="#EF4444", hover_color="#B91C1C")
            self.interval_entry.configure(state="disabled")
            self.count_entry.configure(state="disabled")
            self.mouse_btn_menu.configure(state="disabled")
            self.click_type_menu.configure(state="disabled")
            self.hotkey_btn.configure(state="disabled")
        else:
            self.status_label.configure(text="Stopped", text_color="#EF4444")
            self.start_btn.configure(text="START", fg_color="#10B981", hover_color="#059669")
            self.interval_entry.configure(state="normal")
            self.mouse_btn_menu.configure(state="normal")
            self.click_type_menu.configure(state="normal")
            self.hotkey_btn.configure(state="normal")
            self._on_mode_changed()

    def _on_click_callback(self, count: int):
        self.current_clicks = count

    def _on_engine_stopped(self):
        self.after(0, lambda: self._update_status(running=False))

    def _poll_updates(self):
        self.clicks_label.configure(text=f"Clicks: {self.current_clicks}")
        self.after(50, self._poll_updates)

    def _load_settings_to_ui(self):
        interval = self._config.get("interval", 100)
        self.interval_entry.delete(0, "end")
        self.interval_entry.insert(0, str(interval))
        
        unit = self._config.get("interval_unit", "ms")
        self.interval_unit.set(unit if unit in ["ms", "sec"] else "ms")
        
        btn = self._config.get("mouse_button", "left")
        self.mouse_btn_var.set({"left":"Left","right":"Right","middle":"Middle"}.get(btn, "Left"))
        
        c_type = self._config.get("click_type", "single")
        self.click_type_var.set("Double" if c_type == "double" else "Single")
        
        mode = self._config.get("click_mode", "continuous")
        self.click_mode_var.set(mode)
        self._on_mode_changed()
        
        count = self._config.get("click_count", 10)
        self.count_entry.delete(0, "end")
        self.count_entry.insert(0, str(count))
        
        pos_mode = self._config.get("position_mode", "cursor")
        self.pos_mode_var.set(pos_mode)
        self._on_position_mode_changed()
        
        fx, fy = self._config.get("fixed_x", 0), self._config.get("fixed_y", 0)
        self.pos_x_entry.delete(0, "end"); self.pos_x_entry.insert(0, str(fx))
        self.pos_y_entry.delete(0, "end"); self.pos_y_entry.insert(0, str(fy))
        
        hk = self._config.get("hotkey", "f6")
        self.hotkey_btn.configure(text=f"Hotkey: {hk.upper()}")
        self.hotkey_manager.set_hotkey(hk)
        
        app_mode = self._config.get("appearance_mode", "dark")
        ctk.set_appearance_mode(app_mode)
        self.theme_selector.set(app_mode.capitalize())

    def _save_settings_from_ui(self):
        try: self._config.set("interval", float(self.interval_entry.get()))
        except ValueError: pass
        self._config.set("interval_unit", self.interval_unit.get())
        self._config.set("mouse_button", self.mouse_btn_var.get().lower())
        self._config.set("click_type", "double" if self.click_type_var.get() == "Double" else "single")
        self._config.set("click_mode", self.click_mode_var.get())
        try: self._config.set("click_count", int(self.count_entry.get()))
        except ValueError: pass
        self._config.set("position_mode", self.pos_mode_var.get())
        try:
            self._config.set("fixed_x", int(self.pos_x_entry.get()))
            self._config.set("fixed_y", int(self.pos_y_entry.get()))
        except ValueError: pass
        
        hk_text = self.hotkey_btn.cget("text").replace("Hotkey: ", "").lower()
        self._config.set("hotkey", hk_text)
        self._config.save()

    def _on_close(self):
        self._save_settings_from_ui()
        self.click_engine.stop()
        self.hotkey_manager.stop()
        if self._mouse_listener: self._mouse_listener.stop()
        self.destroy()

def main():
    app = ClickForgeApp()
    app.mainloop()

if __name__ == "__main__":
    main()
