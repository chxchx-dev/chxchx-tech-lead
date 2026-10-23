"""Composes the safe, project-local preparation flow used by Studio quick start."""

from __future__ import annotations

from dataclasses import dataclass

from .bootstrap import initialize_project
from .mods import ModApplyResult, apply_mods, has_claude_project
from .models import ProjectInfo
from .trust import trust_project


@dataclass(frozen=True)
class ProjectPreparationResult:
    actions: tuple[str, ...]
    packs: tuple[str, ...]
    skills: tuple[str, ...]
    mods: tuple[str, ...]
    backups: tuple[str, ...]
    warnings: tuple[str, ...]
    dry_run: bool


def prepare_project(
    info: ProjectInfo,
    *,
    dry_run: bool = False,
    trust: bool = False,
) -> ProjectPreparationResult:
    """Initialize project guidance, apply detected skills/packs, sync, and optionally trust."""
    from ..skills import SkillRegistry, TechPackRegistry, enable_skills, enabled_skills
    from ..skills import sync_project_skills

    claude_enabled = has_claude_project(info.root)
    initialized = initialize_project(info, dry_run=dry_run)
    skill_registry = SkillRegistry()
    recommendations = skill_registry.recommendations(info.root)
    recommended_names = tuple(item.skill.name for item in recommendations)
    detected_packs = TechPackRegistry().detect(info.root)
    packs = tuple(item.pack.name for item in detected_packs)
    pack_skills = tuple(
        dict.fromkeys(skill for match in detected_packs for skill in match.pack.skills)
    )
    selected_names = tuple(dict.fromkeys((*recommended_names, *pack_skills)))
    current = set(enabled_skills(info.root))
    to_enable = tuple(name for name in selected_names if name not in current)

    actions = [
        item for item in initialized.actions
        if item not in {"Todo está actualizado", "No se realizaron cambios (--dry-run)"}
    ]
    if to_enable:
        actions.append("Habilitar skills recomendadas y de los Tech Packs: " + ", ".join(to_enable))
    if selected_names or current:
        actions.append("Sincronizar las instrucciones de skills con AGENTS.md")
    mod_plan: ModApplyResult | None = None
    warnings = list(initialized.warnings)
    if claude_enabled:
        try:
            mod_plan = apply_mods(info.root, dry_run=True)
        except (OSError, ValueError) as exc:
            warnings.append(f"Mods de Claude Code omitidos: {exc}")
        if mod_plan and mod_plan.installed:
            actions.append("Instalar Mods recomendados de Claude Code: " + ", ".join(mod_plan.installed))
    if trust:
        actions.append("Asegurar la confianza local del proyecto al terminar")

    backups: list[str] = []
    if initialized.backup:
        backups.append(str(initialized.backup))

    if dry_run:
        return ProjectPreparationResult(
            tuple(actions or ["Todo está actualizado"]),
            packs,
            selected_names,
            mod_plan.installed if mod_plan else (),
            tuple(backups),
            tuple(warnings),
            True,
        )

    selection = enable_skills(info.root, selected_names, registry=skill_registry)
    if selection.backup:
        backups.append(str(selection.backup))
    if selected_names or enabled_skills(info.root):
        synced = sync_project_skills(info.root, registry=skill_registry)
        if synced.backup:
            backups.append(str(synced.backup))
    mods: tuple[str, ...] = ()
    if mod_plan and mod_plan.installed:
        applied = apply_mods(info.root, mod_plan.installed)
        mods = applied.installed
        if mods:
            actions.append("Mods de Claude Code instalados en .claude/skills")
        backups.extend(applied.backups)
    if trust and trust_project(info.root):
        actions.append("Proyecto marcado como confiable en este equipo")
    if not actions:
        actions.append("Todo está actualizado")

    return ProjectPreparationResult(
        tuple(actions), packs, selected_names, mods, tuple(backups), tuple(warnings), False
    )
