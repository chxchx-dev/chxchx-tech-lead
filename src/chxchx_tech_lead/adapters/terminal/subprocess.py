from __future__ import annotations

from pathlib import Path
from typing import Callable, Sequence

from ...core.runner import CommandResult, run


class SubprocessAdapter:
    """Fallback sin sesiones persistentes; ejecuta comandos directamente."""

    def __init__(self, runner: Callable[..., CommandResult] = run):
        self._runner = runner

    def available(self) -> bool:
        return True

    def session_exists(self, name: str) -> bool:
        return False

    def create_session(
        self,
        name: str,
        cwd: Path,
        dry_run: bool = False,
        layout: str | None = None,
    ) -> CommandResult:
        return CommandResult([], 0, "fallback: no se crea una sesión persistente", "", skipped=True)

    def attach_session(self, name: str, dry_run: bool = False) -> CommandResult:
        return CommandResult([], 1, "", "El fallback no admite attach de sesiones")

    def close_session(self, name: str, dry_run: bool = False) -> CommandResult:
        return CommandResult([], 0, "fallback: no hay sesión que cerrar", "", skipped=True)

    def list_panes(self, name: str, dry_run: bool = False) -> CommandResult:
        return CommandResult([], 1, "", "El fallback no tiene panes persistentes", skipped=True)

    def run_in_session(
        self,
        name: str,
        command: Sequence[str],
        cwd: Path,
        pane_name: str | None = None,
        direction: str | None = None,
        dry_run: bool = False,
    ) -> CommandResult:
        return self._runner([str(part) for part in command], dry_run=dry_run, cwd=cwd)
