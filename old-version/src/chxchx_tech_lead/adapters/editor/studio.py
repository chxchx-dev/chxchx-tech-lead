from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
from typing import Callable

from ...core.runner import CommandResult


class StudioAdapter:
    """Launch the installed ChxChx Studio without blocking the caller."""

    def __init__(
        self,
        *,
        lookup: Callable[[str], str | None] = shutil.which,
        popen: Callable[..., object] = subprocess.Popen,
    ):
        self._lookup = lookup
        self._popen = popen

    def available(self) -> bool:
        return self._executable() is not None

    def open_project(self, path: Path, dry_run: bool = False) -> CommandResult:
        target = path.expanduser().resolve()
        executable = self._executable()
        if not target.is_dir():
            return CommandResult(["chxchx-studio-dev", str(target)], 2, "", f"No existe el proyecto: {target}")
        command = [str(executable) if executable is not None else "chxchx-studio-dev", str(target)]
        if dry_run:
            return CommandResult(command, 0, "DRY RUN", "")
        if executable is None:
            return CommandResult(
                command,
                127,
                "",
                "ChxChx Studio no está instalado. Instálalo con el instalador de Studio y vuelve a intentarlo.",
            )

        try:
            options: dict[str, object] = {
                "cwd": str(target),
                "stdin": subprocess.DEVNULL,
                "stdout": subprocess.DEVNULL,
                "stderr": subprocess.DEVNULL,
                "close_fds": True,
            }
            if os.name == "nt":
                options["creationflags"] = (
                    getattr(subprocess, "DETACHED_PROCESS", 0)
                    | getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
                )
            else:
                options["start_new_session"] = True
            self._popen(command, **options)
        except OSError as exc:
            return CommandResult(command, 1, "", f"No pude iniciar ChxChx Studio: {exc}")
        return CommandResult(command, 0, "ChxChx Studio iniciado", "")

    def open_file(
        self,
        path: Path,
        line: int | None = None,
        column: int | None = None,
        dry_run: bool = False,
    ) -> CommandResult:
        target = path.expanduser().resolve()
        if not target.is_file():
            return CommandResult(["chxchx-studio-dev", str(target)], 2, "", f"No existe el archivo: {target}")
        return CommandResult(
            ["chxchx-studio-dev", str(target)],
            2,
            "",
            "Studio abre proyectos; la apertura directa de archivos todavía no está disponible.",
        )

    def _executable(self) -> Path | None:
        if os.name == "nt":
            root = Path(os.environ.get("LOCALAPPDATA", "")) / "Programs" / "ChxChxStudio"
            installs = [
                item / "bin" / "chxchx-studio.exe"
                for item in root.glob("dev-*")
                if (item / ".chxchx-managed").is_file()
            ] if root.is_dir() else []
            installs = [item for item in installs if item.is_file()]
            if installs:
                return max(installs, key=lambda item: (item.stat().st_mtime, item.parent.parent.name))

        command_name = "chxchx-studio-dev.exe" if os.name == "nt" else "chxchx-studio-dev"
        discovered = self._lookup(command_name)
        candidates = [Path(discovered)] if discovered else []
        if os.name != "nt":
            candidates.append(Path.home() / ".local" / "bin" / command_name)
        return next((item for item in candidates if item.is_file()), None)
