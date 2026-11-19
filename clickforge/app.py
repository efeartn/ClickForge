"""Application entry point and lightweight CLI self-check for ClickForge."""

from __future__ import annotations

import os
import sys
import tempfile
from typing import Optional, Sequence

from clickforge import __app_name__, __codename__, __version__


def _safe_print(msg: str) -> None:
    """Print safely even under Windows ``console=False`` builds with null stdout."""
    if sys.stdout is not None:
        print(msg)


def run_smoke_test() -> int:
    """Verify core subsystems and bundled assets without opening a GUI window."""
    from clickforge.core.clicker import ClickEngine, ClickSettings
    from clickforge.core.config import Config
    from clickforge.core.hotkeys import TriggerListener
    from clickforge.core.system import resource_path
    from clickforge.ui.icons import ICON_NAMES, header_icon

    icon_png = resource_path("assets", "icon.png")
    if not os.path.isfile(icon_png):
        raise FileNotFoundError(f"Missing bundled asset: {icon_png}")

    with tempfile.TemporaryDirectory() as tmp:
        cfg = Config(path=os.path.join(tmp, "config.json"))
        if cfg.get("hotkey") != "f6":
            raise RuntimeError("Unexpected default hotkey")

    settings = ClickSettings(interval_ms=10.0).sanitized()
    if settings.interval_ms < 1.0:
        raise RuntimeError("Invalid sanitized interval")

    ClickEngine()
    listener = TriggerListener(
        binding="vk117",
        on_press=lambda: None,
        on_release=lambda: None,
        on_panic=lambda: None,
        on_recorded=lambda _k: None,
    )
    if listener.binding != "f6":
        raise RuntimeError(f"Expected f6 binding, got {listener.binding!r}")

    for name in ICON_NAMES:
        header_icon(name, active=False)
        header_icon(name, active=True)

    _safe_print(f"{__app_name__} v{__version__} ({__codename__}) smoke-test OK")
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = sys.argv[1:] if argv is None else list(argv)
    if "--version" in args:
        _safe_print(f"{__app_name__} v{__version__} ({__codename__})")
        return 0
    if "--smoke-test" in args:
        return run_smoke_test()

    from clickforge.ui.main_window import ClickForgeWindow

    app = ClickForgeWindow()
    app.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
