from __future__ import annotations

from pathlib import Path
from typing import Protocol

from ...core.runner import CommandResult


class EditorAdapter(Protocol):
    def available(self) -> bool: ...

    def open_project(self, path: Path, dry_run: bool = False) -> CommandResult: ...

    def open_file(
        self,
        path: Path,
        line: int | None = None,
        column: int | None = None,
        dry_run: bool = False,
    ) -> CommandResult: ...
