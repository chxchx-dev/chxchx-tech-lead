from __future__ import annotations

import re

from rich.theme import Theme


COLORS = {
    "canvas": "#282a36",
    "surface": "#2e303d",
    "raised": "#303240",
    "selection": "#444758",
    "border": "#777b8e",
    "foreground": "#ececf1",
    "muted": "#aeb1c2",
    "accent": "#ddd6ff",
    "success": "#8bbdad",
    "warning": "#e5c07b",
    "error": "#d49ca6",
}

_TUI_COLOR_MAP = {
    "#282a36": "canvas",
    "#20222c": "canvas",
    "#1d1f29": "canvas",
    "#2e303d": "surface",
    "#303240": "raised",
    "#353746": "raised",
    "#3a3d4d": "raised",
    "#444758": "selection",
    "#555868": "border",
    "#555970": "selection",
    "#777b8e": "border",
    "#818598": "border",
    "#aeb1c2": "muted",
    "#d8dae4": "foreground",
    "#ececf1": "foreground",
    "#ffffff": "foreground",
    "#ddd6ff": "accent",
    "#d6caff": "accent",
    "#b4a5f2": "accent",
    "#b9ded1": "success",
    "#8bbdad": "success",
    "#f1c3cb": "error",
    "#d49ca6": "error",
}


def apply_tui_palette(css: str) -> str:
    """Resolve legacy TUI hex colors through the shared design tokens."""
    return re.sub(
        r"#[0-9a-fA-F]{6}",
        lambda match: COLORS[_TUI_COLOR_MAP[match.group().lower()]],
        css,
    )


def cli_theme() -> Theme:
    """Keep terminal output aligned with the TUI status and accent colors."""
    return Theme(
        {
            "green": COLORS["success"],
            "yellow": COLORS["warning"],
            "red": COLORS["error"],
            "cyan": "#9ec9e2",
            "magenta": COLORS["accent"],
            "white": COLORS["foreground"],
        }
    )
