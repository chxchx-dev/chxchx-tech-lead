from __future__ import annotations

from pathlib import Path
from typing import Protocol, Sequence

from ...core.runner import CommandResult


class TerminalWorkspaceAdapter(Protocol):
    def available(self) -> bool: ...

    def session_exists(self, name: str) -> bool: ...

    def create_session(
        self,
        name: str,
        cwd: Path,
        dry_run: bool = False,
        layout: str | None = None,
    ) -> CommandResult: ...

    def attach_session(self, name: str, dry_run: bool = False) -> CommandResult: ...

    def close_session(self, name: str, dry_run: bool = False) -> CommandResult: ...

    def list_panes(self, name: str, dry_run: bool = False) -> CommandResult: ...

    def open_terminal_pane(self, name: str, cwd: Path, dry_run: bool = False) -> CommandResult: ...

    def run_in_session(
        self,
        name: str,
        command: Sequence[str],
        cwd: Path,
        pane_name: str | None = None,
        direction: str | None = None,
        dry_run: bool = False,
    ) -> CommandResult: ...
