from __future__ import annotations

from chichan_tech_lead.core.runner import CommandResult, executable, run


INSTALLABLE = {
    "basic-memory": ["uv", "tool", "install", "-p", "3.12", "basic-memory"],
    "serena": ["uv", "tool", "install", "-p", "3.13", "serena-agent"],
}

INSTALLED_EXECUTABLES = {
    "basic-memory": ("basic-memory", "bm"),
    "serena": ("serena",),
}


def install_tool(name: str, dry_run: bool = False):
    if name not in INSTALLABLE:
        raise KeyError(name)
    if any(executable(command) is not None for command in INSTALLED_EXECUTABLES[name]):
        return CommandResult(INSTALLABLE[name], 0, "already installed", "", skipped=True)
    if executable("uv") is None and not dry_run:
        raise RuntimeError("uv no está instalado o no está disponible en PATH")
    return run(INSTALLABLE[name], dry_run=dry_run)
