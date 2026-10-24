"""Application entry point for ClickForge."""

from __future__ import annotations

from clickforge.ui.main_window import ClickForgeWindow

# Backward-compatible alias
ClickForgeApp = ClickForgeWindow


def main() -> None:
    app = ClickForgeWindow()
    app.mainloop()


if __name__ == "__main__":
    main()
