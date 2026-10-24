"""Interactive screen coordinate picker overlay.

Opens a translucent full-desktop overlay with a crosshair cursor and a live
``X, Y`` readout pill that follows the pointer.  Because the overlay window
captures the mouse click itself, picking a coordinate never accidentally
clicks whatever window is underneath.  Pressing ``Esc`` or right-clicking
cancels the pick.
"""

from __future__ import annotations

import tkinter as tk
from typing import Callable, Optional, Tuple

from clickforge.core.system import IS_WINDOWS, virtual_screen_bounds
from clickforge.ui.theme import Fonts


class CoordinatePicker:
    """Modal fullscreen crosshair overlay that returns ``(x, y)`` or ``None``."""

    def __init__(
        self,
        parent: tk.Misc,
        fonts: Fonts,
        on_done: Callable[[Optional[Tuple[int, int]]], None],
    ) -> None:
        self._parent = parent
        self._fonts = fonts
        self._on_done = on_done
        self._finished = False
        self._top: Optional[tk.Toplevel] = None

    def start(self) -> None:
        top = tk.Toplevel(self._parent)
        self._top = top
        top.withdraw()
        top.overrideredirect(True)
        top.attributes("-topmost", True)

        bounds = virtual_screen_bounds()
        if bounds is not None:
            left, top_y, width, height = bounds
        else:
            left, top_y = 0, 0
            width = top.winfo_screenwidth()
            height = top.winfo_screenheight()

        self._origin_x = left
        self._origin_y = top_y
        top.geometry(f"{width}x{height}+{left}+{top_y}")

        try:
            top.attributes("-alpha", 0.22)
        except tk.TclError:
            pass

        canvas = tk.Canvas(
            top,
            bg="#09090D",
            highlightthickness=1,
            highlightbackground="#7C3AED",
            cursor="crosshair",
        )
        canvas.pack(fill="both", expand=True)
        self._canvas = canvas
        self._width = width
        self._height = height

        # Crosshair guide lines
        self._h_line = canvas.create_line(0, 0, width, 0, fill="#A78BFA", width=1, dash=(4, 4))
        self._v_line = canvas.create_line(0, 0, 0, height, fill="#A78BFA", width=1, dash=(4, 4))
        self._ring = canvas.create_oval(-8, -8, 8, 8, outline="#7C3AED", width=2)

        # Instruction banner at top center
        banner_w = 420
        banner_x = width // 2
        canvas.create_rectangle(
            banner_x - banner_w // 2,
            24,
            banner_x + banner_w // 2,
            60,
            fill="#16161A",
            outline="#7C3AED",
            width=1,
        )
        canvas.create_text(
            banner_x,
            42,
            text="Click anywhere to pick X, Y   •   Right-click or Esc to cancel",
            fill="#FFFFFF",
            font=(self._fonts.ui, 10, "bold"),
        )

        # Floating coordinate readout pill near cursor
        self._pill_bg = canvas.create_rectangle(0, 0, 130, 28, fill="#16161A", outline="#A78BFA", width=1)
        self._pill_txt = canvas.create_text(
            0, 0, text="X: 0  Y: 0", fill="#FFFFFF", font=(self._fonts.mono, 10, "bold")
        )

        canvas.bind("<Motion>", self._on_motion)
        canvas.bind("<ButtonPress-1>", self._on_left_click)
        canvas.bind("<ButtonPress-3>", self._on_cancel)
        top.bind("<Escape>", self._on_cancel)
        top.bind("<Return>", self._on_enter)

        top.deiconify()
        top.lift()
        top.focus_force()
        if IS_WINDOWS:
            try:
                top.grab_set()
            except tk.TclError:
                pass

        # Position initial crosshair at current pointer location
        px = top.winfo_pointerx()
        py = top.winfo_pointery()
        self._update_guides(px, py)

    def _update_guides(self, screen_x: int, screen_y: int) -> None:
        if self._top is None or self._finished:
            return
        cx = screen_x - self._origin_x
        cy = screen_y - self._origin_y
        self._canvas.coords(self._h_line, 0, cy, self._width, cy)
        self._canvas.coords(self._v_line, cx, 0, cx, self._height)
        self._canvas.coords(self._ring, cx - 7, cy - 7, cx + 7, cy + 7)

        # Keep pill inside screen edges
        pill_w, pill_h = 148, 28
        ox = cx + 16 if cx + 16 + pill_w < self._width else cx - 16 - pill_w
        oy = cy + 16 if cy + 16 + pill_h < self._height else cy - 16 - pill_h
        self._canvas.coords(self._pill_bg, ox, oy, ox + pill_w, oy + pill_h)
        self._canvas.coords(self._pill_txt, ox + pill_w // 2, oy + pill_h // 2)
        self._canvas.itemconfigure(self._pill_txt, text=f"X: {screen_x}   Y: {screen_y}")

    def _on_motion(self, event: tk.Event) -> None:
        self._update_guides(event.x_root, event.y_root)

    def _on_left_click(self, event: tk.Event) -> None:
        self._finish((int(event.x_root), int(event.y_root)))

    def _on_enter(self, _event: tk.Event) -> None:
        if self._top is not None:
            self._finish((int(self._top.winfo_pointerx()), int(self._top.winfo_pointery())))

    def _on_cancel(self, _event: Optional[tk.Event] = None) -> None:
        self._finish(None)

    def _finish(self, coords: Optional[Tuple[int, int]]) -> None:
        if self._finished:
            return
        self._finished = True
        if self._top is not None:
            try:
                self._top.grab_release()
            except tk.TclError:
                pass
            try:
                self._top.destroy()
            except tk.TclError:
                pass
            self._top = None
        self._on_done(coords)
