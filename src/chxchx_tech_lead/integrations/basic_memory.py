from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.project_config import memory_project_name
from chxchx_tech_lead.core.runner import executable, run


def available() -> bool:
    return executable("basic-memory") is not None or executable("bm") is not None


def command_name() -> str:
    return "basic-memory" if executable("basic-memory") else "bm"


def ensure_project(info: ProjectInfo, dry_run: bool = False):
    if not available():
        return None
    cmd = command_name()
    name = memory_project_name(info)
    memory_path = info.root / ".ai" / "memory"

    check = run([cmd, "project", "info", name, "--json"], dry_run=False)
    if check.returncode == 0:
        check.skipped = True
        return check

    added = run([cmd, "project", "add", name, str(memory_path)], dry_run=dry_run)
    if added.returncode == 0 or dry_run:
        return added

    # Some Basic Memory versions report an existing project as a failed add.
    # Confirm existence before surfacing the error to keep init idempotent.
    after_add = run([cmd, "project", "info", name, "--json"], dry_run=False)
    if after_add.returncode == 0:
        after_add.skipped = True
        return after_add
    return added


def mcp_command(info: ProjectInfo) -> list[str]:
    return [command_name(), "mcp", "--project", memory_project_name(info)]
