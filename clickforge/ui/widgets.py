"""Small styled building blocks on top of CustomTkinter."""

from __future__ import annotations

from typing import Callable, Dict, Optional, Sequence

import customtkinter as ctk

from clickforge.ui import theme as t
from clickforge.ui.theme import Fonts


class Typography:
    """CTkFont factory bound to the resolved font families."""

    def __init__(self, fonts: Fonts) -> None:
        self.fonts = fonts
        self.body = ctk.CTkFont(family=fonts.ui, size=13)
        self.body_strong = ctk.CTkFont(family=fonts.ui, size=13, weight="bold")
        self.label = ctk.CTkFont(family=fonts.ui, size=12)
        self.caption = ctk.CTkFont(family=fonts.ui, size=11)
        self.section = ctk.CTkFont(family=fonts.ui, size=11, weight="bold")
        self.title = ctk.CTkFont(family=fonts.ui_display, size=15, weight="bold")
        self.button = ctk.CTkFont(family=fonts.ui, size=14, weight="bold")
        self.mono = ctk.CTkFont(family=fonts.mono, size=13)
        self.mono_small = ctk.CTkFont(family=fonts.mono, size=11)
        self.mono_large = ctk.CTkFont(family=fonts.mono, size=26, weight="bold")
        self.icon = ctk.CTkFont(family=fonts.icons, size=13) if fonts.icons else self.body


class Card(ctk.CTkFrame):
    """Bordered surface with an optional section title and header widget."""

    def __init__(self, master, typo: Typography, title: Optional[str] = None, **kw) -> None:
        super().__init__(
            master, fg_color=t.SURFACE, border_color=t.BORDER, border_width=1,
            corner_radius=t.RADIUS, **kw,
        )
        self.header = ctk.CTkFrame(self, fg_color="transparent")
        if title:
            self.header.pack(fill="x", padx=14, pady=(12, 0))
            ctk.CTkLabel(
                self.header, text=title.upper(), font=typo.section,
                text_color=t.TEXT_FAINT, height=18, anchor="w",
            ).pack(side="left")
        self.body = ctk.CTkFrame(self, fg_color="transparent")
        self.body.pack(fill="both", expand=True, padx=14, pady=(8 if title else 12, 12))


class FormRow(ctk.CTkFrame):
    """``label | content`` row with a fixed label column."""

    LABEL_WIDTH = 76

    def __init__(self, master, typo: Typography, label: str = "", pady=(0, 8)) -> None:
        super().__init__(master, fg_color="transparent")
        self.pack(fill="x", pady=pady)
        self.label = ctk.CTkLabel(
            self, text=label, font=typo.label, text_color=t.TEXT_MUTED,
            width=self.LABEL_WIDTH, anchor="w", height=t.CONTROL_H,
        )
        self.label.pack(side="left")
        self.content = ctk.CTkFrame(self, fg_color="transparent")
        self.content.pack(side="left", fill="x", expand=True)

    def set_enabled(self, enabled: bool) -> None:
        self.label.configure(text_color=t.TEXT_MUTED if enabled else t.TEXT_FAINT)


class Segmented(ctk.CTkSegmentedButton):
    """Neutral segmented control mapping display labels to stable keys."""

    def __init__(
        self, master, typo: Typography, options: Dict[str, str],
        command: Optional[Callable[[str], None]] = None, width: int = 0,
    ) -> None:
        self._to_key = {label: key for key, label in options.items()}
        self._to_label = dict(options)
        self._user_command = command
        super().__init__(
            master, values=list(options.values()), height=t.CONTROL_H,
            corner_radius=t.RADIUS_SM, border_width=2, font=typo.label,
            fg_color=t.FIELD, unselected_color=t.FIELD, unselected_hover_color=t.HOVER,
            selected_color=t.SEG_SELECTED, selected_hover_color=t.SEG_SELECTED_HOVER,
            text_color=t.TEXT, text_color_disabled=t.TEXT_FAINT,
            dynamic_resizing=False, command=self._dispatch,
            **({"width": width} if width else {}),
        )

    def _dispatch(self, label: str) -> None:
        if self._user_command:
            self._user_command(self._to_key.get(label, label))

    def get_key(self) -> str:
        return self._to_key.get(self.get(), "")

    def set_key(self, key: str) -> None:
        if key in self._to_label:
            self.set(self._to_label[key])

    def set_enabled(self, enabled: bool) -> None:
        self.configure(state="normal" if enabled else "disabled")


class NumberEntry(ctk.CTkEntry):
    """Monospace numeric entry with keystroke filtering and disabled styling."""

    def __init__(self, master, typo: Typography, allow: Callable[[str], bool], width: int = 72) -> None:
        super().__init__(
            master, width=width, height=t.CONTROL_H, corner_radius=t.RADIUS_SM,
            border_width=1, fg_color=t.FIELD, border_color=t.BORDER_STRONG,
            text_color=t.TEXT, font=typo.mono, justify="center",
        )
        vcmd = (self.register(lambda proposed: allow(proposed)), "%P")
        self._entry.configure(validate="key", validatecommand=vcmd)
        self._enabled = True
        self._entry.bind("<FocusIn>", lambda e: self._focus(True), add="+")
        self._entry.bind("<FocusOut>", lambda e: self._focus(False), add="+")

    @property
    def inner(self):
        return self._entry

    def _focus(self, focused: bool) -> None:
        if self._enabled:
            self.configure(border_color=t.ACCENT if focused else t.BORDER_STRONG)

    def get_text(self) -> str:
        return self.get().strip()

    def set_text(self, value: str) -> None:
        state = self.cget("state")
        if state == "disabled":
            self.configure(state="normal")
        self.delete(0, "end")
        self.insert(0, value)
        if state == "disabled":
            self.configure(state="disabled")

    def flash_error(self) -> None:
        self.configure(border_color=t.DANGER)
        self.after(900, lambda: self._focus(self.focus_get() == self._entry))

    def set_enabled(self, enabled: bool) -> None:
        self._enabled = enabled
        self.configure(
            state="normal" if enabled else "disabled",
            fg_color=t.FIELD if enabled else t.FIELD_DISABLED,
            text_color=t.TEXT if enabled else t.TEXT_FAINT,
            border_color=t.BORDER_STRONG if enabled else t.BORDER,
        )


def ghost_button(master, typo: Typography, text: str, command, width: int = 64, font=None) -> ctk.CTkButton:
    """Low-emphasis bordered button with a visible hover state."""
    return ctk.CTkButton(
        master, text=text, width=width, height=t.CONTROL_H, corner_radius=t.RADIUS_SM,
        border_width=1, border_color=t.BORDER_STRONG, fg_color="transparent",
        hover_color=t.HOVER, text_color=t.TEXT, text_color_disabled=t.TEXT_FAINT,
        font=font or typo.label, command=command,
    )


def accent_button(master, typo: Typography, text: str, command, width: int = 64, height: int = t.CONTROL_H,
                  font=None, radius: int = t.RADIUS_SM) -> ctk.CTkButton:
    return ctk.CTkButton(
        master, text=text, width=width, height=height, corner_radius=radius,
        fg_color=t.ACCENT, hover_color=t.ACCENT_HOVER, text_color=t.TEXT_ON_ACCENT,
        text_color_disabled=t.TEXT_FAINT, font=font or typo.body_strong, command=command,
    )


def set_accent_enabled(button: ctk.CTkButton, enabled: bool) -> None:
    button.configure(
        state="normal" if enabled else "disabled",
        fg_color=t.ACCENT if enabled else t.ACCENT_DISABLED,
    )


class StatusBadge(ctk.CTkFrame):
    """Non-interactive pill: colored dot + state label."""

    def __init__(self, master, typo: Typography) -> None:
        super().__init__(
            master, fg_color=t.FIELD, border_color=t.BORDER, border_width=1,
            corner_radius=12, height=26,
        )
        self.dot = ctk.CTkFrame(self, width=8, height=8, corner_radius=4, fg_color=t.TEXT_FAINT)
        self.dot.pack(side="left", padx=(10, 6), pady=9)
        self.text = ctk.CTkLabel(self, text="Stopped", font=typo.caption, text_color=t.TEXT_MUTED, height=20)
        self.text.pack(side="left", padx=(0, 10), pady=3)

    def set_running(self, running: bool) -> None:
        if running:
            self.configure(fg_color=t.SUCCESS_SOFT, border_color=t.SUCCESS)
            self.dot.configure(fg_color=t.SUCCESS)
            self.text.configure(text="Running", text_color=t.SUCCESS)
        else:
            self.configure(fg_color=t.FIELD, border_color=t.BORDER)
            self.dot.configure(fg_color=t.TEXT_FAINT)
            self.text.configure(text="Stopped", text_color=t.TEXT_MUTED)


def icon_button(master, typo: Typography, glyph: str, fallback: str, command, width: int = 30) -> ctk.CTkButton:
    """Square header button using Fluent glyphs where available."""
    use_glyph = typo.fonts.icons is not None
    return ctk.CTkButton(
        master, text=glyph if use_glyph else fallback, width=width if use_glyph else 0,
        height=t.CONTROL_H - 2, corner_radius=t.RADIUS_SM, border_width=1,
        border_color=t.BORDER, fg_color="transparent", hover_color=t.HOVER,
        text_color=t.TEXT_MUTED, font=typo.icon if use_glyph else typo.caption, command=command,
    )


def caption(master, typo: Typography, text: str = "", mono: bool = False, **kw) -> ctk.CTkLabel:
    return ctk.CTkLabel(
        master, text=text, font=typo.mono_small if mono else typo.caption,
        text_color=t.TEXT_FAINT, anchor="w", height=16, **kw,
    )


def segmented_options(keys: Sequence[str], labels: Sequence[str]) -> Dict[str, str]:
    return dict(zip(keys, labels))
