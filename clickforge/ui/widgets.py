"""Small styled building blocks on top of CustomTkinter."""

from __future__ import annotations

import os
import re
from typing import Callable, Dict, Optional, Sequence

import tkinter as tk

import customtkinter as ctk

from clickforge.core.system import virtual_screen_bounds
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

        def restore() -> None:
            try:
                focused = self.focus_get() == self._entry
            except (KeyError, tk.TclError):
                focused = False
            self._focus(focused)

        self.after(900, restore)

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


class UpdateBanner(ctk.CTkFrame):
    """Slim, dismissible notice used by the auto-updater.

    Shows one message line, an optional detail line, an optional progress bar
    and up to three action buttons.  The owner decides when to pack it.
    """

    def __init__(self, master, typo: Typography) -> None:
        super().__init__(
            master, fg_color=t.SURFACE, border_color=t.ACCENT, border_width=1,
            corner_radius=t.RADIUS,
        )
        self._typo = typo
        body = ctk.CTkFrame(self, fg_color="transparent")
        body.pack(fill="x", padx=12, pady=10)

        text_col = ctk.CTkFrame(body, fg_color="transparent")
        text_col.pack(side="left", fill="x", expand=True)
        self.title = ctk.CTkLabel(
            text_col, text="", font=typo.body_strong, text_color=t.TEXT, anchor="w", height=18,
        )
        self.title.pack(fill="x")
        self.detail = ctk.CTkLabel(
            text_col, text="", font=typo.caption, text_color=t.TEXT_MUTED, anchor="w",
            height=16, justify="left", wraplength=210,
        )
        self.detail.pack(fill="x")
        self.progress = ctk.CTkProgressBar(
            text_col, height=4, corner_radius=2, fg_color=t.FIELD, progress_color=t.ACCENT,
        )
        self.progress.set(0)

        self._actions = ctk.CTkFrame(body, fg_color="transparent")
        self._actions.pack(side="right", padx=(8, 0))
        self._buttons: list = []

    def show_message(
        self,
        title: str,
        detail: str = "",
        actions: Sequence[tuple] = (),
        progress: Optional[float] = None,
        tone: str = "accent",
    ) -> None:
        """``actions`` is a sequence of ``(label, callback, primary)`` tuples."""
        border = {"accent": t.ACCENT, "danger": t.DANGER, "neutral": t.BORDER}.get(tone, t.ACCENT)
        self.configure(border_color=border)
        self.title.configure(text=title)
        self.detail.configure(text=detail)
        self.detail.pack_forget()
        self.progress.pack_forget()
        if detail:
            self.detail.pack(fill="x")
        if progress is not None:
            self.progress.set(max(0.0, min(1.0, progress)))
            self.progress.pack(fill="x", pady=(6, 0))

        for b in self._buttons:
            b.destroy()
        self._buttons = []
        for label, callback, primary in actions:
            if primary:
                btn = accent_button(
                    self._actions, self._typo, label, callback,
                    width=0, height=26, font=self._typo.label,
                )
            else:
                btn = ghost_button(self._actions, self._typo, label, callback, width=0)
                btn.configure(height=26)
            btn.pack(side="left", padx=(6, 0))
            self._buttons.append(btn)

    def set_progress(self, fraction: float, detail: str) -> None:
        self.progress.set(max(0.0, min(1.0, fraction)))
        self.detail.configure(text=detail)


def format_release_notes(notes: str, max_chars: int = 1200) -> str:
    """Turn GitHub Markdown release notes into clean, readable plain text."""
    lines = []
    for raw in (notes or "").splitlines():
        line = raw.strip()
        if not line:
            if lines and lines[-1] != "":
                lines.append("")
            continue
        line = re.sub(r"^#{1,6}\s+", "", line)
        line = re.sub(r"^[-*+]\s+", "\u2022 ", line)
        line = re.sub(r"\*\*(.+?)\*\*", r"\1", line)
        line = re.sub(r"`(.+?)`", r"\1", line)
        lines.append(line)
    while lines and lines[-1] == "":
        lines.pop()
    text = "\n".join(lines).strip()
    if len(text) > max_chars:
        text = text[: max_chars - 3].rstrip() + "..."
    return text


class _ModalDialog(ctk.CTkToplevel):
    """Shared modal dialog shell matching ClickForge's Obsidian card aesthetic."""

    def __init__(
        self,
        parent,
        typo: Typography,
        *,
        title: str,
        headline: str,
        subhead: str,
        on_cancel: Callable[[], None],
        logo_image: Optional[ctk.CTkImage] = None,
        ico_path: Optional[str] = None,
    ) -> None:
        super().__init__(parent)
        self.withdraw()
        self.title(title)
        self.resizable(False, False)
        self.configure(fg_color=t.BG)
        self.transient(parent)

        self._on_cancel = on_cancel
        self._closed = False
        self._parent_ref = parent

        try:
            if parent.attributes("-topmost"):
                self.attributes("-topmost", True)
        except Exception:
            pass

        self._ico_job: Optional[str] = None
        if ico_path and os.path.exists(ico_path):
            def _apply_ico() -> None:
                self._ico_job = None
                if not self.winfo_exists():
                    return
                try:
                    self.iconbitmap(ico_path)
                    self.wm_iconbitmap(ico_path)
                except Exception:
                    pass

            _apply_ico()
            self._ico_job = self.after(240, _apply_ico)

        card = ctk.CTkFrame(
            self,
            fg_color=t.SURFACE,
            border_color=t.BORDER,
            border_width=1,
            corner_radius=t.RADIUS,
        )
        card.pack(fill="both", expand=True, padx=14, pady=14)

        self._body = ctk.CTkFrame(card, fg_color="transparent")
        self._body.pack(fill="both", expand=True, padx=16, pady=16)

        hdr = ctk.CTkFrame(self._body, fg_color="transparent")
        hdr.pack(fill="x", pady=(0, 10))
        if logo_image is not None:
            ctk.CTkLabel(hdr, text="", image=logo_image).pack(
                side="left", anchor="n", padx=(0, 12), pady=(2, 0)
            )

        title_col = ctk.CTkFrame(hdr, fg_color="transparent")
        title_col.pack(side="left", fill="x", expand=True)
        ctk.CTkLabel(
            title_col,
            text=headline,
            font=typo.title,
            text_color=t.TEXT,
            anchor="w",
        ).pack(fill="x")
        ctk.CTkLabel(
            title_col,
            text=subhead,
            font=typo.body,
            text_color=t.TEXT_MUTED,
            anchor="w",
            justify="left",
            wraplength=316,
        ).pack(fill="x", pady=(4, 0))

    def _finalize(self) -> None:
        self.protocol("WM_DELETE_WINDOW", self._cancel_and_close)
        self.bind("<Escape>", lambda _e: self._cancel_and_close())
        self._center_on_parent(self._parent_ref)
        self.deiconify()
        self.lift()
        try:
            self.grab_set()
        except tk.TclError:
            pass

    def _cancel_and_close(self) -> None:
        self.close()
        self._on_cancel()

    def _center_on_parent(self, parent) -> None:
        self.update_idletasks()
        dw = self.winfo_reqwidth()
        dh = self.winfo_reqheight()
        pw = parent.winfo_width()
        ph = parent.winfo_height()
        px = parent.winfo_rootx()
        py = parent.winfo_rooty()
        bounds = virtual_screen_bounds()
        if bounds is not None:
            vleft, vtop, vw, vh = bounds
        else:
            vleft, vtop = 0, 0
            vw, vh = self.winfo_screenwidth(), self.winfo_screenheight()
        x = px + (pw - dw) // 2 if pw > 10 else vleft + (vw - dw) // 2
        y = py + (ph - dh) // 2 if ph > 10 else vtop + (vh - dh) // 2
        x = max(vleft + 16, min(x, max(vleft + 16, vleft + vw - dw - 16)))
        y = max(vtop + 16, min(y, max(vtop + 16, vtop + vh - dh - 16)))
        self.geometry(f"+{x}+{y}")

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        if self._ico_job is not None:
            try:
                self.after_cancel(self._ico_job)
            except tk.TclError:
                pass
            self._ico_job = None
        try:
            if self.winfo_exists():
                self.grab_release()
                self.destroy()
        except tk.TclError:
            pass


class UpdatePromptDialog(_ModalDialog):
    """Themed prompt asking whether the user wants to install an available update."""

    def __init__(
        self,
        parent,
        typo: Typography,
        *,
        headline: str,
        subhead: str,
        notes: str = "",
        primary_label: str = "Update Now",
        on_update: Callable[[], None],
        on_later: Callable[[], None],
        on_skip: Callable[[], None],
        logo_image: Optional[ctk.CTkImage] = None,
        ico_path: Optional[str] = None,
    ) -> None:
        super().__init__(
            parent,
            typo,
            title="ClickForge Update",
            headline=headline,
            subhead=subhead,
            on_cancel=on_later,
            logo_image=logo_image,
            ico_path=ico_path,
        )
        self._on_update = on_update
        self._on_skip = on_skip

        cleaned_notes = format_release_notes(notes)
        if cleaned_notes:
            ctk.CTkLabel(
                self._body,
                text="WHAT'S NEW",
                font=typo.section,
                text_color=t.TEXT_FAINT,
                anchor="w",
                height=16,
            ).pack(fill="x", pady=(2, 4))
            notes_box = ctk.CTkTextbox(
                self._body,
                width=360,
                height=118,
                fg_color=t.FIELD,
                border_color=t.BORDER,
                border_width=1,
                corner_radius=t.RADIUS_SM,
                text_color=t.TEXT_MUTED,
                font=typo.caption,
                wrap="word",
            )
            notes_box.pack(fill="x", pady=(0, 14))
            notes_box.insert("1.0", cleaned_notes)
            notes_box.configure(state="disabled")

        footer = ctk.CTkFrame(self._body, fg_color="transparent")
        footer.pack(fill="x")

        skip_btn = ctk.CTkButton(
            footer,
            text="Skip Version",
            width=0,
            height=30,
            corner_radius=t.RADIUS_SM,
            border_width=0,
            fg_color="transparent",
            hover_color=t.HOVER,
            text_color=t.TEXT_FAINT,
            font=typo.caption,
            command=self._choose_skip,
        )
        skip_btn.pack(side="left")

        self.update_now_btn = accent_button(
            footer,
            typo,
            primary_label,
            self._choose_update,
            width=112,
            height=30,
            font=typo.label,
        )
        self.update_now_btn.pack(side="right", padx=(8, 0))

        self.later_btn = ghost_button(
            footer,
            typo,
            "Not Now",
            self._cancel_and_close,
            width=76,
        )
        self.later_btn.configure(height=30)
        self.later_btn.pack(side="right")

        self._finalize()

    def _choose_update(self) -> None:
        self.close()
        self._on_update()

    def _choose_skip(self) -> None:
        self.close()
        self._on_skip()


class SetupPromptDialog(_ModalDialog):
    """Modal prompt to install ClickForge into Windows Programs & Start Menu."""

    def __init__(
        self,
        parent,
        typo: Typography,
        *,
        headline: str,
        subhead: str,
        details: str,
        primary_label: str = "Install Now",
        show_desktop_option: bool = True,
        on_confirm: Callable[[bool], None],
        on_cancel: Optional[Callable[[], None]] = None,
        logo_image: Optional[ctk.CTkImage] = None,
        ico_path: Optional[str] = None,
    ) -> None:
        super().__init__(
            parent,
            typo,
            title="ClickForge Setup",
            headline=headline,
            subhead=subhead,
            on_cancel=on_cancel or (lambda: None),
            logo_image=logo_image,
            ico_path=ico_path,
        )
        self._on_confirm = on_confirm

        info = ctk.CTkLabel(
            self._body,
            text=details,
            font=typo.caption,
            text_color=t.TEXT_MUTED,
            anchor="w",
            justify="left",
            wraplength=360,
        )
        info.pack(fill="x", pady=(2, 10))

        self.desktop_var = tk.BooleanVar(value=True)
        if show_desktop_option:
            ctk.CTkCheckBox(
                self._body,
                text="Create Desktop shortcut",
                variable=self.desktop_var,
                font=typo.label,
                text_color=t.TEXT,
                fg_color=t.ACCENT,
                hover_color=t.ACCENT_HOVER,
                border_color=t.BORDER_STRONG,
                checkbox_width=18,
                checkbox_height=18,
                corner_radius=4,
            ).pack(anchor="w", pady=(0, 14))

        footer = ctk.CTkFrame(self._body, fg_color="transparent")
        footer.pack(fill="x")

        self.confirm_btn = accent_button(
            footer,
            typo,
            primary_label,
            self._choose_confirm,
            width=128,
            height=30,
            font=typo.label,
        )
        self.confirm_btn.pack(side="right", padx=(8, 0))

        self.cancel_btn = ghost_button(
            footer,
            typo,
            "Cancel",
            self._cancel_and_close,
            width=76,
        )
        self.cancel_btn.configure(height=30)
        self.cancel_btn.pack(side="right")

        self._finalize()

    def _choose_confirm(self) -> None:
        create_desktop = bool(self.desktop_var.get())
        self.close()
        self._on_confirm(create_desktop)
