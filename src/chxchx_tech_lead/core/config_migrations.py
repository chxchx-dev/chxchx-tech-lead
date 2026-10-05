from __future__ import annotations

import json
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .models import ProjectInfo
from ..workspace.models import WorkspaceConfig, WorkspaceConfigError

CURRENT_CONFIG_VERSION = 2


class ConfigMigrationError(ValueError):
    """Indica que una configuración no puede migrarse de forma segura."""


@dataclass(frozen=True, slots=True)
class ConfigMigrationResult:
    path: Path
    source_version: int | None
    target_version: int
    changed: bool
    content: str


def default_config_data(info: ProjectInfo) -> dict[str, Any]:
    from .project_config import memory_project_name
    from .project_commands import suggested_processes

    return {
        "version": CURRENT_CONFIG_VERSION,
        "profile": info.profile_name,
        "memory_project": memory_project_name(info),
        "memory_path": ".ai/memory",
        "workspace": {
            "name": info.name,
            "adapter": "zellij",
            "editor": "sublime",
            "auto_open_editor": False,
            "auto_attach": True,
            "auto_start": False,
            "header": {
                "enabled": True,
                "label": "chxchx-tech",
                "logo": "",
                "template": "{logo} {label} | {project} | {profile}",
            },
            "layout": {
                "orientation": "horizontal",
            },
            "resources": {
                "warn_memory_percent": 70,
                "critical_memory_percent": 85,
                "warn_swap_percent": 40,
                "max_agents": 2,
            },
            "processes": suggested_processes(info),
            "agents": [
                {
                    "id": "codex",
                    "command": ["codex"],
                    "cwd": ".",
                    "auto_start": False,
                },
                {
                    "id": "claude",
                    "command": ["claude"],
                    "cwd": ".",
                    "auto_start": False,
                },
            ],
            "presets": {
                "default": ["codex", "claude"],
            },
        },
    }


def _read(path: Path) -> dict[str, Any]:
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ConfigMigrationError(f"No se pudo leer {path}: {exc}") from exc
    if not isinstance(raw, dict):
        raise ConfigMigrationError(f"{path} debe contener una tabla TOML")
    return raw


def _validate(data: dict[str, Any], project_root: Path) -> None:
    if data.get("version") != CURRENT_CONFIG_VERSION:
        raise ConfigMigrationError("La configuración no está en la versión 2")
    try:
        WorkspaceConfig.from_mapping(data.get("workspace"), project_root=project_root)
    except WorkspaceConfigError as exc:
        raise ConfigMigrationError(str(exc)) from exc


def migrate_data(data: dict[str, Any], info: ProjectInfo) -> tuple[dict[str, Any], int]:
    source_version = data.get("version")
    if source_version == 1:
        migrated = dict(data)
        migrated["version"] = CURRENT_CONFIG_VERSION
        workspace = migrated.get("workspace")
        if workspace is None:
            workspace = default_config_data(info)["workspace"]
        elif not isinstance(workspace, dict):
            raise ConfigMigrationError("workspace debe ser una tabla")
        migrated["workspace"] = workspace
    elif source_version == CURRENT_CONFIG_VERSION:
        migrated = dict(data)
    else:
        raise ConfigMigrationError(f"Versión de configuración no soportada: {source_version!r}")
    _validate(migrated, info.root)
    return migrated, source_version


def _toml_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    raise ConfigMigrationError(f"Valor TOML no soportado: {value!r}")


def _toml_array(values: list[str]) -> str:
    return "[" + ", ".join(_toml_value(value) for value in values) + "]"


def _render_table(lines: list[str], name: str, values: dict[str, Any]) -> None:
    lines.append(f"[{name}]")
    for key, value in values.items():
        if isinstance(value, (str, int, bool)):
            lines.append(f"{key} = {_toml_value(value)}")
    lines.append("")


def render_config(data: dict[str, Any]) -> str:
    """Renderiza el subconjunto v2 generado por ChxChx de forma estable."""
    lines = [f"version = {_toml_value(data['version'])}"]
    for key in ("profile", "memory_project", "memory_path"):
        if key in data:
            lines.append(f"{key} = {_toml_value(data[key])}")
    lines.append("")

    workspace = data["workspace"]
    _render_table(
        lines,
        "workspace",
        {key: workspace[key] for key in ("name", "adapter", "editor", "auto_open_editor", "auto_attach", "auto_start") if key in workspace},
    )
    resources = workspace.get("resources", {})
    header = workspace.get("header", {})
    _render_table(
        lines,
        "workspace.header",
        {key: header[key] for key in ("enabled", "label", "logo", "template") if key in header},
    )
    layout = workspace.get("layout", {})
    _render_table(
        lines,
        "workspace.layout",
        {key: layout[key] for key in ("orientation",) if key in layout},
    )
    presets = workspace.get("presets", {})
    if presets:
        lines.append("[workspace.presets]")
        for name, agents in presets.items():
            if isinstance(name, str) and isinstance(agents, list) and all(isinstance(agent, str) for agent in agents):
                lines.append(f"{name} = {_toml_array(agents)}")
        lines.append("")
    _render_table(
        lines,
        "workspace.resources",
        {key: resources[key] for key in ("warn_memory_percent", "critical_memory_percent", "warn_swap_percent", "max_agents") if key in resources},
    )
    for section, items in (("workspace.processes", workspace.get("processes", [])), ("workspace.agents", workspace.get("agents", []))):
        for item in items:
            lines.append(f"[[{section}]]")
            for key, value in item.items():
                if isinstance(value, list):
                    lines.append(f"{key} = [{', '.join(_toml_value(part) for part in value)}]")
                elif isinstance(value, (str, int, bool)):
                    lines.append(f"{key} = {_toml_value(value)}")
            lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def migrate_project_config(info: ProjectInfo, dry_run: bool = False) -> ConfigMigrationResult:
    path = info.root / ".ai" / "chxchx-tech.toml"
    if path.exists():
        raw = _read(path)
        migrated, source_version = migrate_data(raw, info)
    else:
        migrated = default_config_data(info)
        _validate(migrated, info.root)
        source_version = None
    current = path.read_text(encoding="utf-8") if path.exists() else None
    # Una configuración v2 existente es propiedad del proyecto: se valida, pero
    # no se reescribe ni se normalizan campos desconocidos durante `init`.
    content = current if source_version == CURRENT_CONFIG_VERSION and current is not None else render_config(migrated)
    changed = current != content
    if changed and not dry_run:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    return ConfigMigrationResult(path, source_version, CURRENT_CONFIG_VERSION, changed, content)
