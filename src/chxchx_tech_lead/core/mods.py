"""Curated, project-local Claude Code workflow Mods."""

from __future__ import annotations

import os
import shutil
import tempfile
import tomllib
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from .detector import detect_project


@dataclass(frozen=True, slots=True)
class Mod:
    name: str
    description: str
    stacks: tuple[str, ...]
    languages: tuple[str, ...]
    always_recommend: bool
    instructions: str
    infrastructure: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class ModRecommendation:
    mod: Mod
    reasons: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ModApplyResult:
    installed: tuple[str, ...]
    unchanged: tuple[str, ...]
    backups: tuple[str, ...]
    dry_run: bool


class ModRegistry:
    """Loads trusted first-party instructions bundled with ChxChx."""

    def __init__(self, root: Path | None = None) -> None:
        self.root = root or Path(__file__).parent / "mod_catalog"

    def list(self) -> list[Mod]:
        return [self._load(path) for path in sorted(self.root.glob("*/mod.toml"))]

    def get(self, name: str) -> Mod | None:
        key = name.casefold()
        return next((item for item in self.list() if item.name.casefold() == key), None)

    def recommendations(self, project: Path) -> list[ModRecommendation]:
        info = detect_project(project)
        stacks, languages, infrastructure = set(info.stacks), set(info.languages), set(info.infrastructure)
        result = []
        for mod in self.list():
            reasons = [f"stack detectado: {stack}" for stack in mod.stacks if stack in stacks]
            reasons.extend(f"lenguaje detectado: {lang}" for lang in mod.languages if lang in languages)
            reasons.extend(f"infraestructura detectada: {item}" for item in mod.infrastructure if item in infrastructure)
            if mod.always_recommend:
                reasons.append("flujo general recomendado")
            if reasons:
                result.append(ModRecommendation(mod, tuple(reasons)))
        return result

    @staticmethod
    def _load(manifest: Path) -> Mod:
        values = tomllib.loads(manifest.read_text(encoding="utf-8"))
        name, description = values.get("name"), values.get("description")
        if not isinstance(name, str) or not isinstance(description, str):
            raise ValueError(f"Mod inválido: {manifest}")
        return Mod(
            name=name,
            description=description,
            stacks=tuple(values.get("stacks", [])),
            languages=tuple(values.get("languages", [])),
            infrastructure=tuple(values.get("infrastructure", [])),
            always_recommend=bool(values.get("always_recommend", False)),
            instructions=manifest.with_name("SKILL.md").read_text(encoding="utf-8").strip(),
        )


def project_mod_status(project: Path, registry: ModRegistry | None = None) -> list[dict[str, object]]:
    """Describe catalog, recommendations and safe install state without file contents."""
    registry = registry or ModRegistry()
    recommended = {item.mod.name: item for item in registry.recommendations(project)}
    result = []
    for mod in registry.list():
        target = project / ".claude" / "skills" / mod.name / "SKILL.md"
        try:
            content = target.read_text(encoding="utf-8") if target.is_file() else ""
        except (OSError, UnicodeDecodeError):
            content = ""
        managed = f"<!-- chxchx-mod:{mod.name} -->" in content
        recommendation = recommended.get(mod.name)
        result.append({
            "name": mod.name,
            "description": mod.description,
            "recommended": recommendation is not None,
            "reasons": list(recommendation.reasons) if recommendation else [],
            "applied": managed,
            "conflict": target.exists() and not managed,
        })
    return result


def _configured_agent_commands(root: Path) -> list[str]:
    """Executable names of the agents in `.ai/chxchx-tech.toml`."""
    try:
        config = tomllib.loads((root / ".ai" / "chxchx-tech.toml").read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError):
        return []
    workspace = config.get("workspace", {})
    agents = workspace.get("agents", []) if isinstance(workspace, dict) else []
    names: list[str] = []
    for agent in agents if isinstance(agents, list) else []:
        command = agent.get("command", []) if isinstance(agent, dict) else []
        parts = command if isinstance(command, list) else [command]
        if parts:
            name = Path(str(parts[0]).replace("\\", "/")).name.casefold()
            names.append(name.rsplit(".", 1)[0] if name.endswith((".exe", ".cmd")) else name)
    return names


def has_claude_project(root: Path) -> bool:
    """Recognize configured Claude agents and existing Claude project context."""
    if (root / ".claude").is_dir():
        return True
    claude_context = root / "CLAUDE.md"
    try:
        if claude_context.is_file() and "<!-- chxchx-tech:start claude-rules -->" not in claude_context.read_text(encoding="utf-8"):
            return True
    except (OSError, UnicodeDecodeError):
        pass
    return "claude" in _configured_agent_commands(root)


def skill_agents(root: Path) -> tuple[str, ...]:
    """Agents of the project that read Mods as skills: Claude, Codex and OpenCode."""
    commands = set(_configured_agent_commands(root))
    found = []
    if has_claude_project(root):
        found.append("claude")
    if "codex" in commands or (root / ".agents").is_dir():
        found.append("codex")
    if "opencode" in commands or (root / ".opencode").is_dir():
        found.append("opencode")
    return tuple(found)


def skill_roots(project: Path) -> tuple[tuple[str, Path], ...]:
    """Where the Mods are written. Claude Code and OpenCode read `.claude/skills`; Codex reads `.agents/skills`."""
    roots = [("claude", project / ".claude" / "skills")]
    if "codex" in skill_agents(project):
        roots.append(("agents", project / ".agents" / "skills"))
    return tuple(roots)


def apply_mods(
    project: Path,
    names: tuple[str, ...] | None = None,
    *,
    dry_run: bool = False,
    registry: ModRegistry | None = None,
) -> ModApplyResult:
    """Install recommended or selected instruction-only Mods under .claude/skills."""
    registry = registry or ModRegistry()
    catalog = {item.name: item for item in registry.list()}
    if names is None:
        selected = tuple(item.mod for item in registry.recommendations(project))
    else:
        missing = [name for name in names if name not in catalog]
        if missing:
            raise ValueError(f"Mods desconocidos: {', '.join(missing)}")
        selected = tuple(catalog[name] for name in dict.fromkeys(names))

    targets = [(mod, label, root / mod.name / "SKILL.md", _render_mod(mod))
               for mod in selected for label, root in skill_roots(project)]
    for mod, _, target, content in targets:
        if not target.exists():
            continue
        existing = target.read_text(encoding="utf-8")
        if existing == content:
            continue
        if f"<!-- chxchx-mod:{mod.name} -->" not in existing:
            raise ValueError(f"{target} existe y no está administrado por ChxChx; no se sobrescribió.")

    current = [(mod, label, target, content) for mod, label, target, content in targets
               if not (target.exists() and target.read_text(encoding="utf-8") == content)]
    pending_names = tuple(dict.fromkeys(mod.name for mod, *_ in current))
    unchanged = tuple(mod.name for mod in selected if mod.name not in pending_names)
    if dry_run or not current:
        return ModApplyResult(pending_names, unchanged, (), dry_run)

    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    backup_root = project / ".ai" / "backups" / "claude-mods" / stamp
    backups: list[str] = []
    for mod, label, target, _ in current:
        if target.exists():
            backup = backup_root / label / mod.name / "SKILL.md"
            backup.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target, backup)
            backups.append(str(backup))
    for _, _, target, content in current:
        target.parent.mkdir(parents=True, exist_ok=True)
        _atomic_write(target, content)
    return ModApplyResult(pending_names, unchanged, tuple(backups), False)


def remove_mods(
    project: Path,
    names: tuple[str, ...],
    *,
    dry_run: bool = False,
    registry: ModRegistry | None = None,
) -> ModApplyResult:
    """Remove only selected first-party files, saving a backup before deletion."""
    known = {item.name for item in (registry or ModRegistry()).list()}
    missing = [name for name in names if name not in known]
    if missing:
        raise ValueError(f"Mods desconocidos: {', '.join(missing)}")
    selected = tuple(dict.fromkeys(names))
    targets = [(name, label, root / name / "SKILL.md") for name in selected for label, root in _all_skill_roots(project)]
    for name, _, target in targets:
        if not target.exists():
            continue
        if f"<!-- chxchx-mod:{name} -->" not in target.read_text(encoding="utf-8"):
            raise ValueError(f"{target} no está administrado por ChxChx; no se eliminó.")
    present = [(name, label, target) for name, label, target in targets if target.exists()]
    present_names = tuple(dict.fromkeys(name for name, _, _ in present))
    absent = tuple(name for name in selected if name not in present_names)
    if dry_run or not present:
        return ModApplyResult(present_names, absent, (), dry_run)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    backups: list[str] = []
    for name, label, target in present:
        backup = project / ".ai" / "backups" / "claude-mods" / stamp / label / name / "SKILL.md"
        backup.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(target, backup)
        backups.append(str(backup))
        target.unlink()
        target.parent.rmdir()
    return ModApplyResult(present_names, absent, tuple(backups), False)


def _all_skill_roots(project: Path) -> tuple[tuple[str, Path], ...]:
    """Removal looks in every root, so Mods are retired even after an agent was dropped from the project."""
    return (("claude", project / ".claude" / "skills"), ("agents", project / ".agents" / "skills"))


def _render_mod(mod: Mod) -> str:
    return (
        f"---\nname: {mod.name}\ndescription: {mod.description}\n---\n\n"
        f"<!-- chxchx-mod:{mod.name} -->\n\n{mod.instructions}\n"
    )


def _atomic_write(target: Path, content: str) -> None:
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, target)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
