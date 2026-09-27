from __future__ import annotations

from pathlib import Path
from typing import Callable, Sequence

from ...core.runner import CommandResult, executable, run


class SublimeAdapter:
    """Abre proyectos y archivos mediante el comando opcional `subl`."""

    def __init__(
        self,
        command: str = "subl",
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
    ):
        self.command = command
        self._runner = runner
        self._lookup = lookup

    def available(self) -> bool:
        return self._lookup(self.command) is not None

    def open_project(self, path: Path, dry_run: bool = False) -> CommandResult:
        target = path.expanduser().resolve()
        if not target.is_dir():
            return CommandResult([self.command, str(target)], 2, "", f"No existe el proyecto: {target}")
        return self._runner([self.command, str(target)], dry_run=dry_run)

    def open_file(
        self,
        path: Path,
        line: int | None = None,
        column: int | None = None,
        dry_run: bool = False,
    ) -> CommandResult:
        target = path.expanduser().resolve()
        if line is not None and line < 1:
            return CommandResult([self.command, str(target)], 2, "", "La línea debe ser mayor que cero")
        if column is not None and column < 1:
            return CommandResult([self.command, str(target)], 2, "", "La columna debe ser mayor que cero")
        location = str(target)
        if line is not None:
            location += f":{line}"
            if column is not None:
                location += f":{column}"
        return self._runner([self.command, location], dry_run=dry_run)
