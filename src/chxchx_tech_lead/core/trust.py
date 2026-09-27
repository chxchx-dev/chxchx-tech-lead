from __future__ import annotations

import hashlib
import json
from pathlib import Path

from .paths import ensure_home, home_dir


def trust_path() -> Path:
    return home_dir() / "trusted_projects.json"


def _digest(project_root: Path) -> str:
    config = project_root / ".ai" / "chxchx-tech.toml"
    if not config.exists():
        return "missing-config"
    return hashlib.sha256(config.read_bytes()).hexdigest()


def load_trust() -> dict[str, str]:
    path = trust_path()
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    if not isinstance(data, dict):
        return {}
    return {key: value for key, value in data.items() if isinstance(key, str) and isinstance(value, str)}


def is_trusted(project_root: Path) -> bool:
    root = project_root.resolve()
    return load_trust().get(str(root)) == _digest(root)


def trust_project(project_root: Path, dry_run: bool = False) -> bool:
    root = project_root.resolve()
    data = load_trust()
    digest = _digest(root)
    if data.get(str(root)) == digest:
        return False
    data[str(root)] = digest
    if not dry_run:
        ensure_home()
        trust_path().write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return True


def untrust_project(project_root: Path, dry_run: bool = False) -> bool:
    root = project_root.resolve()
    data = load_trust()
    if str(root) not in data:
        return False
    del data[str(root)]
    if not dry_run:
        ensure_home()
        trust_path().write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return True
