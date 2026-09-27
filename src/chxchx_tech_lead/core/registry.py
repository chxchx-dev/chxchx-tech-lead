from __future__ import annotations

import json
import hashlib
import re
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


def _slug(value: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9._-]+", "-", value.strip().lower()).strip("-")
    return slug or "project"


def _project_alias(info: ProjectInfo, projects: list[dict], current: dict | None = None) -> str:
    if current and isinstance(current.get("alias"), str) and current["alias"].strip():
        return current["alias"].strip()
    candidate = _slug(info.name)
    occupied = {
        str(project.get("alias"))
        for project in projects
        if project is not current and isinstance(project.get("alias"), str)
    }
    if candidate not in occupied:
        return candidate
    digest = hashlib.sha1(str(info.root).encode("utf-8")).hexdigest()[:6]
    return f"{candidate}-{digest}"


def register_project(info: ProjectInfo, dry_run: bool = False) -> bool:
    data = load_registry()
    projects = data.setdefault("projects", [])
    key = str(info.root)
    current = next((p for p in projects if p.get("path") == key), None)
    payload = {
        "alias": _project_alias(info, projects, current),
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


def resolve_project_reference(reference: str | Path) -> Path:
    """Resolve an existing path or a registered alias/name to a project root."""
    raw = str(reference).strip()
    candidate = Path(raw).expanduser()
    if candidate.exists():
        if not candidate.is_dir():
            raise ValueError(f"La ruta del proyecto no es un directorio: {candidate}")
        return candidate.resolve()

    projects = load_registry().get("projects", [])
    matches = [
        project
        for project in projects
        if raw in {str(project.get("alias", "")), str(project.get("name", ""))}
    ]
    if not matches:
        raise ValueError(f"No existe un proyecto o alias registrado: {raw}")
    if len(matches) > 1:
        raise ValueError(f"El proyecto `{raw}` es ambiguo; usa su alias único")
    path = matches[0].get("path")
    if not isinstance(path, str) or not Path(path).is_dir():
        raise ValueError(f"El proyecto registrado `{raw}` ya no existe en disco")
    return Path(path).resolve()


def set_last_project(path: Path, dry_run: bool = False) -> bool:
    data = load_registry()
    value = str(path.expanduser().resolve())
    changed = data.get("last_project") != value
    if not changed:
        return False
    data["last_project"] = value
    if not dry_run:
        ensure_home()
        registry_path().write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return True


def last_project() -> Path | None:
    value = load_registry().get("last_project")
    if not isinstance(value, str) or not Path(value).is_dir():
        return None
    return Path(value).resolve()
