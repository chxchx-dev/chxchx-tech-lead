from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .backup import backup_project
from .detector import detect_project
from .managed import upsert_managed_block
from .models import ProjectInfo
from .templates import claude_body, project_rule_body


@dataclass(frozen=True, slots=True)
class SyncResult:
    info: ProjectInfo
    changed: bool
    backup: Path | None = None


def sync_project(root: Path, dry_run: bool = False, create_backup: bool = True) -> SyncResult:
    info = detect_project(root)
    agents = info.root / "AGENTS.md"
    claude = info.root / "CLAUDE.md"
    planned_agents = upsert_managed_block(
        agents, "project-rules", project_rule_body(info), dry_run=True
    )
    planned_claude = upsert_managed_block(
        claude, "claude-rules", claude_body(), dry_run=True
    )
    changed = planned_agents or planned_claude
    backup = None
    if changed and not dry_run and create_backup:
        backup = backup_project(info.root)

    actual_agents = upsert_managed_block(
        agents, "project-rules", project_rule_body(info), dry_run=dry_run
    )
    actual_claude = upsert_managed_block(
        claude, "claude-rules", claude_body(), dry_run=dry_run
    )
    return SyncResult(info=info, changed=actual_agents or actual_claude, backup=backup)
