from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass(slots=True)
class ProjectInfo:
    root: Path
    name: str
    stacks: list[str] = field(default_factory=list)
    languages: list[str] = field(default_factory=list)
    package_managers: list[str] = field(default_factory=list)
    infrastructure: list[str] = field(default_factory=list)

    @property
    def profile_name(self) -> str:
        from .profiles import resolve_profile

        return resolve_profile(self.stacks)
