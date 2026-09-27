from __future__ import annotations

from pathlib import Path
from typing import Protocol

from ...core.runner import CommandResult


class DockerAdapter(Protocol):
    def available(self) -> bool: ...

    def compose_available(self) -> bool: ...

    def up(self, dry_run: bool = False) -> CommandResult: ...

    def stop(self, dry_run: bool = False) -> CommandResult: ...

    def down(self, dry_run: bool = False) -> CommandResult: ...

    def ps(self, dry_run: bool = False) -> CommandResult: ...
