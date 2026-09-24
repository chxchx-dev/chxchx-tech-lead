from __future__ import annotations

import hashlib
import shutil
from datetime import datetime
from pathlib import Path

from .paths import ensure_home, home_dir

MANAGED = ["AGENTS.md", "CLAUDE.md", ".ai"]


def _project_key(root: Path) -> str:
    digest = hashlib.sha1(str(root.resolve()).encode("utf-8")).hexdigest()[:8]
    return f"{root.name}-{digest}"


def backup_project(root: Path) -> Path | None:
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    target = ensure_home() / "backups" / _project_key(root) / stamp
    copied = False
    for rel in MANAGED:
        source = root / rel
        if not source.exists():
            continue
        dest = target / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(source, dest)
        else:
            shutil.copy2(source, dest)
        copied = True
    return target if copied else None


def latest_backup(root: Path) -> Path | None:
    base = home_dir() / "backups" / _project_key(root)
    if not base.exists():
        return None
    items = sorted((p for p in base.iterdir() if p.is_dir()), reverse=True)
    return items[0] if items else None


def restore_backup(root: Path, backup: Path) -> Path:
    backed_up = {item.name for item in backup.iterdir()}
    for rel in MANAGED:
        if rel in backed_up:
            continue
        dest = root / rel
        if dest.exists():
            if dest.is_dir():
                shutil.rmtree(dest)
            else:
                dest.unlink()

    for item in backup.iterdir():
        dest = root / item.name
        if dest.exists():
            if dest.is_dir():
                shutil.rmtree(dest)
            else:
                dest.unlink()
        if item.is_dir():
            shutil.copytree(item, dest)
        else:
            shutil.copy2(item, dest)
    return backup


def restore_latest(root: Path) -> Path | None:
    backup = latest_backup(root)
    if backup is None:
        return None
    return restore_backup(root, backup)
