from __future__ import annotations

import hashlib
import re
from pathlib import Path

from .models import ProjectInfo


def _slug(value: str) -> str:
    value = value.lower().strip()
    value = re.sub(r"[^a-z0-9._-]+", "-", value)
    return value.strip("-") or "project"


def memory_project_name(info: ProjectInfo) -> str:
    digest = hashlib.sha1(str(info.root).encode("utf-8")).hexdigest()[:6]
    return f"{_slug(info.name)}-{digest}"


def config_content(info: ProjectInfo) -> str:
    memory_name = memory_project_name(info)
    return (
        "version = 1\n"
        f'profile = "{info.profile_name}"\n'
        f'memory_project = "{memory_name}"\n'
        'memory_path = ".ai/memory"\n'
    )


def project_config_needs_update(info: ProjectInfo) -> bool:
    target = info.root / ".ai" / "chichan.toml"
    expected = config_content(info)
    return not target.exists() or target.read_text(encoding="utf-8") != expected


def ensure_project_config(info: ProjectInfo, dry_run: bool = False) -> Path:
    target = info.root / ".ai" / "chichan.toml"
    expected = config_content(info)
    if not dry_run:
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists() or target.read_text(encoding="utf-8") != expected:
            target.write_text(expected, encoding="utf-8")
        (info.root / ".ai" / "memory").mkdir(parents=True, exist_ok=True)
    return target
