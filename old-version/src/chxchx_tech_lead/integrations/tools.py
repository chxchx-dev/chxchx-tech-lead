from __future__ import annotations

from .base import check


TOOLS = [
    ("uv", "uv"),
    ("Git", "git"),
    ("Basic Memory", "basic-memory"),
    ("Serena", "serena"),
    ("Claude Code", "claude"),
    ("Codex", "codex"),
    ("OpenCode", "opencode"),
    ("Zellij", "zellij"),
    ("Sublime Text", "subl"),
]


def check_tools():
    return [check(name, command) for name, command in TOOLS]
