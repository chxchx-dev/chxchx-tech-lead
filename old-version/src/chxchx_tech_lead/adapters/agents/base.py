from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from ...core.runner import CommandResult


@dataclass(frozen=True, slots=True)
class AgentInfo:
    id: str
    command: str
    available: bool
    version: str | None = None


class AgentAdapter(Protocol):
    def available(self) -> bool: ...

    def version(self) -> str | None: ...

    def status(self) -> AgentInfo: ...

    def start(self, project: Path, session: str | None = None, dry_run: bool = False) -> CommandResult: ...
