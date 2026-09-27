from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from ...core.runner import CommandResult, executable, run


class GitAdapterError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class GitStatus:
    branch: str
    dirty: bool
    changed_count: int
    upstream: str | None = None
    ahead: int = 0
    behind: int = 0


class GitAdapter:
    """Expone únicamente información local de Git; no hace commit ni push."""

    def __init__(
        self,
        project_root: Path,
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
    ):
        self.project_root = project_root.expanduser().resolve()
        self._runner = runner
        self._lookup = lookup

    def available(self) -> bool:
        return self._lookup("git") is not None

    def status(self) -> GitStatus:
        if not self.available():
            raise GitAdapterError("Git no está disponible en PATH")
        branch_result = self._run("branch", "--show-current")
        changes_result = self._run("status", "--porcelain=v1")
        upstream_result = self._run("rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}", allow_failure=True)
        if branch_result.returncode != 0 or changes_result.returncode != 0:
            detail = branch_result.stderr or changes_result.stderr or "la ruta no parece un repositorio Git"
            raise GitAdapterError(detail)
        upstream = upstream_result.stdout or None if upstream_result.returncode == 0 else None
        ahead = behind = 0
        if upstream:
            counts = self._run("rev-list", "--left-right", "--count", "@{upstream}...HEAD", allow_failure=True)
            if counts.returncode == 0:
                values = counts.stdout.split()
                if len(values) == 2 and all(value.isdigit() for value in values):
                    behind, ahead = (int(value) for value in values)
        changed_count = len(changes_result.stdout.splitlines()) if changes_result.stdout else 0
        return GitStatus(
            branch=branch_result.stdout or "HEAD",
            dirty=changed_count > 0,
            changed_count=changed_count,
            upstream=upstream,
            ahead=ahead,
            behind=behind,
        )

    def _run(self, *arguments: str, allow_failure: bool = False) -> CommandResult:
        return self._runner(["git", "-C", str(self.project_root), *arguments], dry_run=False)
