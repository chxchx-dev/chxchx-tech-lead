from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class AgentRuntimeStatus:
    id: str
    command: str
    cwd: str
    available: bool
    version: str | None
    session: str
    pane: str
    preset: str

