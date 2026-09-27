from __future__ import annotations

from typing import Protocol

from .cli import GitStatus


class GitReadOnlyAdapter(Protocol):
    def available(self) -> bool: ...

    def status(self) -> GitStatus: ...
