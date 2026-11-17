from __future__ import annotations

import os
from pathlib import Path
import sys

from chxchx_tech_lead.core.runner import CommandResult, executable, run


INSTALLABLE = {
    "basic-memory": ["uv", "tool", "install", "-p", "3.12", "--prerelease", "allow", "basic-memory"],
    "serena": ["uv", "tool", "install", "-p", "3.13", "serena-agent"],
}

INSTALLED_EXECUTABLES = {
    "basic-memory": ("basic-memory", "bm"),
    "serena": ("serena",),
}


def _tool_bin_directory() -> Path:
    for directory in Path(sys.executable).resolve().parents:
        if (directory / ".chxchx-msi").is_file():
            return directory.parent / "tools"
    configured = os.environ.get("UV_TOOL_BIN_DIR")
    if configured:
        return Path(configured).expanduser()
    return Path.home() / ".local" / "bin"


def _uv_command() -> str | None:
    executable_path = Path(sys.executable).resolve()
    for directory in executable_path.parents:
        if (directory / ".chxchx-msi").is_file():
            bundled = executable_path.parent / "uv.exe"
            if bundled.is_file():
                return str(bundled)
    installed = executable("uv")
    if installed:
        return installed
    directories = [executable_path.parent, Path.home() / ".local" / "bin"]
    for directory in directories:
        for name in ("uv.exe", "uv"):
            candidate = directory / name
            if candidate.is_file():
                return str(candidate)
    return None


def _tool_is_installed(name: str, tool_bin: Path) -> bool:
    return any(
        (tool_bin / candidate).is_file()
        for command in INSTALLED_EXECUTABLES[name]
        for candidate in (command, f"{command}.exe", f"{command}.cmd")
    )


def _with_tool_environment(tool_bin: Path, uv: str, command: list[str]) -> CommandResult:
    old_path = os.environ.get("PATH")
    old_bin = os.environ.get("UV_TOOL_BIN_DIR")
    additions = [str(tool_bin), str(Path(uv).resolve().parent)]
    paths = [*additions, *(old_path or "").split(os.pathsep)]
    os.environ["PATH"] = os.pathsep.join(dict.fromkeys(part for part in paths if part))
    os.environ["UV_TOOL_BIN_DIR"] = str(tool_bin)
    try:
        return run(command, timeout=1200)
    finally:
        if old_path is None:
            os.environ.pop("PATH", None)
        else:
            os.environ["PATH"] = old_path
        if old_bin is None:
            os.environ.pop("UV_TOOL_BIN_DIR", None)
        else:
            os.environ["UV_TOOL_BIN_DIR"] = old_bin


def install_tool(name: str, dry_run: bool = False):
    if name not in INSTALLABLE:
        raise KeyError(name)
    tool_bin = _tool_bin_directory()
    uv = _uv_command()
    if _tool_is_installed(name, tool_bin):
        return CommandResult(INSTALLABLE[name], 0, "already installed", "", skipped=True)
    if uv is None and not dry_run:
        raise RuntimeError("uv no está instalado o no está disponible en PATH")
    command = [*INSTALLABLE[name]]
    if uv:
        command[0] = uv
    if dry_run:
        return run(command, dry_run=True)
    return _with_tool_environment(tool_bin, uv, command)
