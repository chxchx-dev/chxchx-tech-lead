from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path
import tempfile
import tomllib

from ..core.backup import backup_project
from ..core.managed import upsert_managed_block
from .registry import SkillRegistry

SELECTION_FILE = Path(".ai/chxchx-skills.toml")
CONTEXT_FILE = Path(".ai/SKILLS.md")


@dataclass(frozen=True)
class ProjectSkillResult:
    changed: bool
    enabled: tuple[str, ...]
    backup: Path | None = None


def enabled_skills(project: Path) -> list[str]:
    path = project / SELECTION_FILE
    if not path.exists():
        return []
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"No se pudo leer {path}: {exc}") from exc
    if set(data) - {"version", "enabled"} or data.get("version") != 1:
        raise ValueError(f"Configuración de skills no compatible: {path}")
    enabled = data.get("enabled", [])
    if not isinstance(enabled, list) or any(not isinstance(name, str) for name in enabled):
        raise ValueError(f"El campo enabled debe ser una lista de nombres en {path}")
    return sorted(set(enabled))


def set_skill_enabled(
    project: Path,
    name: str,
    enabled: bool,
    *,
    registry: SkillRegistry | None = None,
    dry_run: bool = False,
) -> ProjectSkillResult:
    registry = registry or SkillRegistry()
    skill = registry.get(name)
    if skill is None:
        raise ValueError(f"Skill desconocida: {name}")
    if enabled:
        return enable_skills(project, (skill.name,), registry=registry, dry_run=dry_run)

    selected = set(enabled_skills(project))
    if skill.name not in selected:
        return ProjectSkillResult(False, tuple(sorted(selected)))
    selected.remove(skill.name)
    names = tuple(sorted(selected))
    if dry_run:
        return ProjectSkillResult(True, names)
    backup = _store_selection(project, names)
    return ProjectSkillResult(True, names, backup)


def enable_skills(
    project: Path,
    names: tuple[str, ...] | list[str],
    *,
    registry: SkillRegistry | None = None,
    dry_run: bool = False,
) -> ProjectSkillResult:
    registry = registry or SkillRegistry()
    skills = []
    for name in names:
        skill = registry.get(name)
        if skill is None:
            raise ValueError(f"Skill desconocida: {name}")
        skills.append(skill.name)
    selected = set(enabled_skills(project))
    updated = tuple(sorted(selected.union(skills)))
    if updated == tuple(sorted(selected)):
        return ProjectSkillResult(False, updated)
    if dry_run:
        return ProjectSkillResult(True, updated)
    backup = _store_selection(project, updated)
    return ProjectSkillResult(True, updated, backup)


def _store_selection(project: Path, names: tuple[str, ...]) -> Path | None:
    backup = backup_project(project)
    target = project / SELECTION_FILE
    target.parent.mkdir(parents=True, exist_ok=True)
    content = "version = 1\nenabled = [\n" + "".join(f'  "{item}",\n' for item in names) + "]\n"
    descriptor, temporary = tempfile.mkstemp(prefix="chxchx-skills-", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(content)
        os.replace(temporary, target)
    except Exception:
        Path(temporary).unlink(missing_ok=True)
        raise
    return backup


def sync_project_skills(
    project: Path,
    *,
    registry: SkillRegistry | None = None,
    dry_run: bool = False,
) -> ProjectSkillResult:
    registry = registry or SkillRegistry()
    names = enabled_skills(project)
    selected = []
    for name in names:
        skill = registry.get(name)
        if skill is None:
            raise ValueError(f"Skill habilitada que ya no existe en el registro: {name}")
        selected.append(skill)

    body = "\n\n".join(f"## {skill.name}\n\n{skill.instructions}" for skill in selected)
    if not body:
        body = "No hay skills habilitadas para este proyecto."
    context_path = project / CONTEXT_FILE
    agents_path = project / "AGENTS.md"
    agents_body = (
        "Consulta `.ai/SKILLS.md` al trabajar en tareas cubiertas por esas skills. "
        f"Skills habilitadas: {', '.join(names) or 'ninguna'}."
    )
    changed = upsert_managed_block(context_path, "active-skills", body, dry_run=True)
    changed = upsert_managed_block(agents_path, "project-skills", agents_body, dry_run=True) or changed
    if not changed or dry_run:
        return ProjectSkillResult(changed, tuple(names))

    backup = backup_project(project)
    upsert_managed_block(context_path, "active-skills", body)
    upsert_managed_block(agents_path, "project-skills", agents_body)
    return ProjectSkillResult(True, tuple(names), backup)
