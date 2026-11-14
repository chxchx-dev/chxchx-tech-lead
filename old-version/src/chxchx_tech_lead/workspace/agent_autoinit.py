"""Prepare a project minimally when an agent is added before `init`."""

from __future__ import annotations

from pathlib import Path

from ..core.bootstrap import initialize_project
from .project_context import project_for_path


def needs_minimal_init(root: Path) -> bool:
    return not (root.expanduser().resolve() / ".ai" / "chxchx-tech.toml").is_file()


def minimal_init(root: Path, *, dry_run: bool) -> list[str]:
    """Run the compact init (only `.ai/chxchx-tech.toml` and `.ai/memory`) and return its actions."""
    result = initialize_project(project_for_path(root), dry_run=dry_run, minimal=True)
    return list(result.actions)
