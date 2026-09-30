from __future__ import annotations

import os
from pathlib import Path
from typing import Callable

from ...core.runner import CommandResult, executable, run
from ..terminal.base import TerminalWorkspaceAdapter
from .base import AgentInfo


class CliAgentAdapter:
    def __init__(
        self,
        agent_id: str,
        command: str,
        arguments: list[str] | None = None,
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
        terminal: TerminalWorkspaceAdapter | None = None,
    ):
        self.agent_id = agent_id
        self.command = command
        self.arguments = list(arguments or [])
        self._runner = runner
        self._lookup = lookup
        self._terminal = terminal

    def available(self) -> bool:
        return self._lookup(self.command) is not None

    def version(self) -> str | None:
        resolved = self._lookup(self.command)
        if not resolved:
            return None
        if os.name == "nt" and Path(resolved).suffix.lower() in {".bat", ".cmd"}:
            return None
        try:
            result = self._runner([self.command, "--version"], dry_run=False)
        except OSError:
            return None
        if result.returncode != 0:
            return None
        return result.stdout or result.stderr or None

    def status(self) -> AgentInfo:
        return AgentInfo(self.agent_id, self.command, self.available(), self.version())

    def start(
        self,
        project: Path,
        session: str | None = None,
        dry_run: bool = False,
        direction: str | None = None,
    ) -> CommandResult:
        root = project.expanduser().resolve()
        if not root.is_dir():
            return CommandResult([self.command], 2, "", f"No existe el proyecto: {root}")
        if not self.available() and not dry_run:
            return CommandResult([self.command], 127, "", f"No se encontró `{self.command}` en PATH")
        command = [self.command, *self.arguments]
        if session is not None:
            if self._terminal is None:
                return CommandResult(command, 2, "", "No hay adapter de terminal para iniciar el agente en sesión")
            return self._terminal.run_in_session(
                session,
                command,
                root,
                pane_name=self.agent_id,
                direction=direction,
                dry_run=dry_run,
            )
        return self._runner(command, dry_run=dry_run, cwd=root, interactive=True)
