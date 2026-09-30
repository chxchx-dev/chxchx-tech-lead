from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .config_validation import (
    WorkspaceConfigError,
    as_bool as _as_bool,
    as_string as _as_string,
    command as _command,
    port as _port,
    relative_path as _relative_path,
    validate_header_template,
)


@dataclass(frozen=True, slots=True)
class HeaderConfig:
    """Configuración del header compacto persistente de la sesión."""

    enabled: bool = True
    label: str = "chxchx-tech"
    logo: str = ""
    template: str = "{logo} {label} | {project} | {profile}"

    @classmethod
    def from_mapping(cls, raw: Any) -> "HeaderConfig":
        if raw is None:
            raw = {}
        if not isinstance(raw, dict):
            raise WorkspaceConfigError("workspace.header debe ser una tabla")
        template = _as_string(
            raw.get("template", "{logo} {label} | {project} | {profile}"),
            "workspace.header.template",
        )
        validate_header_template(template)
        logo = raw.get("logo", "")
        if not isinstance(logo, str):
            raise WorkspaceConfigError("workspace.header.logo debe ser un texto")
        return cls(
            enabled=_as_bool(raw.get("enabled"), "workspace.header.enabled", True),
            label=_as_string(raw.get("label", "chxchx-tech"), "workspace.header.label"),
            logo=logo,
            template=template,
        )


@dataclass(frozen=True, slots=True)
class LayoutConfig:
    """Arrangement of agent panes inside the workspace."""

    orientation: str = "horizontal"

    @classmethod
    def from_mapping(cls, raw: Any) -> "LayoutConfig":
        if raw is None:
            raw = {}
        if not isinstance(raw, dict):
            raise WorkspaceConfigError("workspace.layout debe ser una tabla")
        orientation = _as_string(
            raw.get("orientation", "horizontal"), "workspace.layout.orientation"
        ).lower()
        if orientation not in {"horizontal", "vertical"}:
            raise WorkspaceConfigError(
                "workspace.layout.orientation debe ser `horizontal` o `vertical`"
            )
        return cls(orientation=orientation)


@dataclass(frozen=True, slots=True)
class ResourceConfig:
    warn_memory_percent: int = 75
    critical_memory_percent: int = 90
    warn_swap_percent: int = 40

    @classmethod
    def from_mapping(cls, raw: Any) -> "ResourceConfig":
        if raw is None:
            raw = {}
        if not isinstance(raw, dict):
            raise WorkspaceConfigError("workspace.resources debe ser una tabla")
        values: dict[str, int] = {}
        for name, default in (
            ("warn_memory_percent", 75),
            ("critical_memory_percent", 90),
            ("warn_swap_percent", 40),
        ):
            value = raw.get(name, default)
            if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 100:
                raise WorkspaceConfigError(f"workspace.resources.{name} debe estar entre 0 y 100")
            values[name] = value
        if values["critical_memory_percent"] < values["warn_memory_percent"]:
            raise WorkspaceConfigError("critical_memory_percent no puede ser menor que warn_memory_percent")
        return cls(**values)


@dataclass(frozen=True, slots=True)
class ProcessConfig:
    id: str
    label: str
    command: list[str] | str
    cwd: str = "."
    auto_start: bool = False
    restart: str = "never"
    port: int | None = None
    shell: bool = False

    @classmethod
    def from_mapping(cls, raw: Any, index: int) -> "ProcessConfig":
        if not isinstance(raw, dict):
            raise WorkspaceConfigError(f"workspace.processes[{index}] debe ser una tabla")
        process_id = _as_string(raw.get("id"), f"workspace.processes[{index}].id")
        shell = _as_bool(raw.get("shell"), f"workspace.processes[{index}].shell", False)
        restart = _as_string(raw.get("restart", "never"), f"workspace.processes[{index}].restart")
        if restart not in {"never", "on-failure", "always"}:
            raise WorkspaceConfigError(f"restart inválido para {process_id}: {restart}")
        return cls(
            id=process_id,
            label=_as_string(raw.get("label", process_id), f"workspace.processes[{index}].label"),
            command=_command(raw.get("command"), f"workspace.processes[{index}].command", shell),
            cwd=_relative_path(raw.get("cwd", "."), f"workspace.processes[{index}].cwd"),
            auto_start=_as_bool(raw.get("auto_start"), f"workspace.processes[{index}].auto_start", False),
            restart=restart,
            port=_port(raw.get("port"), f"workspace.processes[{index}].port"),
            shell=shell,
        )


@dataclass(frozen=True, slots=True)
class AgentConfig:
    id: str
    command: list[str] | str
    cwd: str = "."
    auto_start: bool = False
    shell: bool = False

    @classmethod
    def from_mapping(cls, raw: Any, index: int) -> "AgentConfig":
        if not isinstance(raw, dict):
            raise WorkspaceConfigError(f"workspace.agents[{index}] debe ser una tabla")
        agent_id = _as_string(raw.get("id"), f"workspace.agents[{index}].id")
        shell = _as_bool(raw.get("shell"), f"workspace.agents[{index}].shell", False)
        return cls(
            id=agent_id,
            command=_command(raw.get("command"), f"workspace.agents[{index}].command", shell),
            cwd=_relative_path(raw.get("cwd", "."), f"workspace.agents[{index}].cwd"),
            auto_start=_as_bool(raw.get("auto_start"), f"workspace.agents[{index}].auto_start", False),
            shell=shell,
        )


@dataclass(frozen=True, slots=True)
class AgentPresetConfig:
    id: str
    agents: tuple[str, ...]


def _presets(raw: Any, agents: tuple[AgentConfig, ...]) -> tuple[AgentPresetConfig, ...]:
    if raw is None:
        return (AgentPresetConfig("default", tuple(agent.id for agent in agents)),)
    if not isinstance(raw, dict):
        raise WorkspaceConfigError("workspace.presets debe ser una tabla")
    known = {agent.id for agent in agents}
    presets: list[AgentPresetConfig] = []
    for preset_id, raw_agents in raw.items():
        if not isinstance(preset_id, str) or not preset_id.strip():
            raise WorkspaceConfigError("workspace.presets contiene un nombre inválido")
        if not isinstance(raw_agents, list) or not all(isinstance(item, str) and item for item in raw_agents):
            raise WorkspaceConfigError(f"workspace.presets.{preset_id} debe ser una lista de agentes")
        unknown = sorted(set(raw_agents) - known)
        if unknown:
            raise WorkspaceConfigError(
                f"workspace.presets.{preset_id} referencia agentes inexistentes: {', '.join(unknown)}"
            )
        presets.append(AgentPresetConfig(preset_id.strip(), tuple(raw_agents)))
    return tuple(presets)


@dataclass(frozen=True, slots=True)
class WorkspaceConfig:
    name: str
    adapter: str = "zellij"
    editor: str = "sublime"
    auto_open_editor: bool = False
    auto_attach: bool = True
    auto_start: bool = False
    header: HeaderConfig = field(default_factory=HeaderConfig)
    layout: LayoutConfig = field(default_factory=LayoutConfig)
    resources: ResourceConfig = field(default_factory=ResourceConfig)
    processes: tuple[ProcessConfig, ...] = ()
    agents: tuple[AgentConfig, ...] = ()
    presets: tuple[AgentPresetConfig, ...] = ()

    @classmethod
    def from_mapping(cls, raw: Any, project_root: Path | None = None) -> "WorkspaceConfig":
        if not isinstance(raw, dict):
            raise WorkspaceConfigError("workspace debe ser una tabla")
        raw_processes = raw.get("processes", [])
        raw_agents = raw.get("agents", [])
        if not isinstance(raw_processes, list):
            raise WorkspaceConfigError("workspace.processes debe ser una lista")
        if not isinstance(raw_agents, list):
            raise WorkspaceConfigError("workspace.agents debe ser una lista")
        processes = tuple(ProcessConfig.from_mapping(item, index) for index, item in enumerate(raw_processes))
        agents = tuple(AgentConfig.from_mapping(item, index) for index, item in enumerate(raw_agents))
        presets = _presets(raw.get("presets"), agents)
        ids = [item.id for item in processes] + [item.id for item in agents]
        duplicates = sorted({item for item in ids if ids.count(item) > 1})
        if duplicates:
            raise WorkspaceConfigError(f"IDs duplicados en workspace: {', '.join(duplicates)}")
        if project_root is not None:
            for item in (*processes, *agents):
                cwd = (project_root / item.cwd).resolve()
                root = project_root.resolve()
                if root not in (cwd, *cwd.parents):
                    raise WorkspaceConfigError(f"cwd fuera del proyecto para {item.id}")
        return cls(
            name=_as_string(raw.get("name"), "workspace.name"),
            adapter=_as_string(raw.get("adapter", "zellij"), "workspace.adapter"),
            editor=_as_string(raw.get("editor", "sublime"), "workspace.editor"),
            auto_open_editor=_as_bool(raw.get("auto_open_editor"), "workspace.auto_open_editor", False),
            auto_attach=_as_bool(raw.get("auto_attach"), "workspace.auto_attach", True),
            auto_start=_as_bool(raw.get("auto_start"), "workspace.auto_start", False),
            header=HeaderConfig.from_mapping(raw.get("header")),
            layout=LayoutConfig.from_mapping(raw.get("layout")),
            resources=ResourceConfig.from_mapping(raw.get("resources")),
            processes=processes,
            agents=agents,
            presets=presets,
        )
