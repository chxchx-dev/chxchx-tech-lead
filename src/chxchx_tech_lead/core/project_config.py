from __future__ import annotations

import hashlib
import re
from pathlib import Path

from .config_migrations import (
    ConfigMigrationError,
    default_config_data,
    migrate_project_config,
    render_config,
)
from .models import ProjectInfo


def _slug(value: str) -> str:
    value = value.lower().strip()
    value = re.sub(r"[^a-z0-9._-]+", "-", value)
    return value.strip("-") or "project"


def memory_project_name(info: ProjectInfo) -> str:
    digest = hashlib.sha1(str(info.root).encode("utf-8")).hexdigest()[:6]
    return f"{_slug(info.name)}-{digest}"


def config_content(info: ProjectInfo) -> str:
    return render_config(default_config_data(info))


def project_config_needs_update(info: ProjectInfo) -> bool:
    target = info.root / ".ai" / "chxchx-tech.toml"
    if not target.exists():
        return True
    try:
        return migrate_project_config(info, dry_run=True).changed
    except ConfigMigrationError:
        return True


def ensure_project_config(info: ProjectInfo, dry_run: bool = False) -> Path:
    target = migrate_project_config(info, dry_run=dry_run).path
    if not dry_run:
        (info.root / ".ai" / "memory").mkdir(parents=True, exist_ok=True)
    return target
