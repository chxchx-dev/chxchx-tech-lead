from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from .models import ProjectInfo
from .paths import ensure_home, home_dir


def registry_path() -> Path:
    return home_dir() / "projects.json"


def load_registry() -> dict:
    path = registry_path()
    if not path.exists():
        return {"projects": []}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {"projects": []}
    if not isinstance(data, dict) or not isinstance(data.get("projects", []), list):
        return {"projects": []}
    data["projects"] = [project for project in data["projects"] if isinstance(project, dict)]
    return data


def register_project(info: ProjectInfo, dry_run: bool = False) -> bool:
    data = load_registry()
    projects = data.setdefault("projects", [])
    key = str(info.root)
    current = next((p for p in projects if p.get("path") == key), None)
    payload = {
        "name": info.name,
        "path": key,
        "profile": info.profile_name,
        "stacks": info.stacks,
    }
    if current:
        changed = any(current.get(k) != v for k, v in payload.items())
        if not changed:
            return False
        current.update(payload)
        current["updated_at"] = datetime.now(timezone.utc).isoformat()
    else:
        payload["updated_at"] = datetime.now(timezone.utc).isoformat()
        projects.append(payload)
        changed = True
    if not dry_run:
        ensure_home()
        registry_path().write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return changed
