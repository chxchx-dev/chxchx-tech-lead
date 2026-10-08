from __future__ import annotations

from pathlib import Path
from typing import Any

from ..core.detector import detect_project
from ..core.registry import load_registry
from ..core.trust import is_trusted
from ..skills import SkillRegistry, TechPackRegistry, enabled_skills
from .process_manager import ManagedProcess, ProcessManager, ProcessStatus
from .resources import ResourceManager, summarize_process_resources
from .manager import WorkspaceManager
from .service import WorkspaceService
from .state import load_state
from .bridge_contracts import PROJECT_STATUS_SCHEMA, SCHEMA_VERSION, ProjectStatusPayload
from .agent_commands import agent_pane_command


def project_status_payload(project_path: Path) -> ProjectStatusPayload:
    """Build the versioned, read-only status contract used by native clients."""
    project = detect_project(project_path)
    service = WorkspaceService(project)
    inspection = WorkspaceManager(project).inspect()
    config = inspection.config
    agents = service.agent_statuses(probe_versions=False) if config is not None else []
    manager = ProcessManager(
        project.root,
        config.processes if config is not None else (),
        trusted=inspection.trusted,
    )
    managed_processes = manager.list(persist=False)
    processes_by_id = {process.id: process for process in managed_processes}
    for process_config in config.processes if config is not None else ():
        processes_by_id.setdefault(
            process_config.id,
            ManagedProcess(
                id=process_config.id,
                label=process_config.label,
                command=process_config.command,
                cwd=(project.root / process_config.cwd).resolve(),
                status=ProcessStatus.STOPPED,
                port=process_config.port,
            ),
        )
    managed_processes = sorted(processes_by_id.values(), key=lambda process: process.id)
    resources = ResourceManager(config.resources if config is not None else None)
    system = resources.system()
    measured_processes = resources.processes(managed_processes)
    process_summary = summarize_process_resources(measured_processes)
    measured_by_id = {item.process_id: item for item in measured_processes}
    registry = load_registry()
    skill_registry = SkillRegistry()
    enabled = set(enabled_skills(project.root))
    recommendations = {
        item.skill.name: list(item.reasons)
        for item in skill_registry.recommendations(project.root)
    }
    skills = [
        {
            "name": skill.name,
            "description": skill.description,
            "enabled": skill.name in enabled,
            "recommended": skill.name in recommendations,
            "reasons": recommendations.get(skill.name, []),
        }
        for skill in skill_registry.list()
    ]
    pack_matches = {
        item.pack.name: list(item.reasons)
        for item in TechPackRegistry().detect(project.root)
    }
    packs = [
        {
            "name": pack.name,
            "description": pack.description,
            "skills": list(pack.skills),
            "recommended": pack.name in pack_matches,
            "reasons": pack_matches.get(pack.name, []),
        }
        for pack in TechPackRegistry().list()
    ]
    last_project = registry.get("last_project")
    agent_config_by_id = {item.id: item for item in config.agents} if config is not None else {}

    def studio_command(agent_id: str, *, new_chat: bool) -> list[str]:
        configured = agent_config_by_id.get(agent_id)
        if configured is None or not isinstance(configured.command, list):
            return []
        try:
            return agent_pane_command(
                agent_id,
                configured.command,
                label=config.header.label,
                logo=config.header.logo,
                new_chat=new_chat,
                project_root=str(project.root),
            )
        except ValueError:
            return []

    registered_projects = []
    for entry in registry.get("projects", []):
        raw_path = entry.get("path")
        if not isinstance(raw_path, str):
            continue
        registered_path = Path(raw_path).expanduser()
        exists = registered_path.is_dir()
        resolved_path = registered_path.resolve() if exists else registered_path
        registered_projects.append({
            "alias": str(entry.get("alias", entry.get("name", ""))),
            "name": str(entry.get("name", registered_path.name)),
            "path": str(resolved_path),
            "profile": str(entry.get("profile", "generic")),
            "exists": exists,
            "trusted": is_trusted(resolved_path) if exists else False,
            "status": load_state(resolved_path).status.value if exists else "MISSING",
            "last_active": exists and last_project == str(resolved_path),
        })

    return {
        "schema": PROJECT_STATUS_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "project": {
            "name": project.name,
            "root": str(project.root),
            "profile": project.profile_name,
            "stacks": project.stacks,
            "languages": project.languages,
        },
        "workspace": {
            "trusted": inspection.trusted,
            "status": inspection.state.status.value,
            "session_name": inspection.state.session_name,
            "config_path": str(inspection.config_path),
            "config_valid": config is not None,
            "configured_agent_count": len(config.agents) if config is not None else 0,
            "configured_process_count": len(config.processes) if config is not None else 0,
            "warnings": list(inspection.warnings),
            "error": inspection.error,
        },
        "registered_projects": registered_projects,
        "skills": skills,
        "enabled_skill_count": len(enabled),
        "packs": packs,
        "agents": [
            {
                "id": agent.id,
                "command": agent.command,
                "arguments": agent_config_by_id[agent.id].command[1:]
                    if agent.id in agent_config_by_id and isinstance(agent_config_by_id[agent.id].command, list)
                    else [],
                "studio_command": studio_command(agent.id, new_chat=False),
                "studio_new_chat_command": studio_command(agent.id, new_chat=True),
                "shell": agent_config_by_id[agent.id].shell if agent.id in agent_config_by_id else False,
                "cwd": agent.cwd,
                "available": agent.available,
                "session": agent.session,
                "pane": agent.pane,
                "preset": agent.preset,
                "version": agent.version,
            }
            for agent in agents
        ],
        "processes": [
            {
                "id": process.id,
                "label": process.label,
                "status": process.status.value,
                "pid": process.pid,
                "port": process.port,
                "rss_bytes": measured_by_id[process.id].rss_bytes,
                "cpu_percent": measured_by_id[process.id].cpu_percent,
            }
            for process in managed_processes
        ],
        "resources": {
            "memory": {
                "total_bytes": system.total_bytes,
                "used_bytes": system.used_bytes,
                "available_bytes": system.available_bytes,
                "percent": system.memory_percent,
            },
            "swap": {
                "total_bytes": system.swap_total_bytes,
                "used_bytes": system.swap_used_bytes,
                "percent": system.swap_percent,
            },
            "cpu_percent": system.cpu_percent,
            "processes": {
                "running_count": process_summary.running_count,
                "rss_bytes": process_summary.rss_bytes,
                "cpu_percent": process_summary.cpu_percent,
                "measured_cpu_count": process_summary.measured_cpu_count,
            },
            "governor": {
                "warning_memory_percent": resources.config.warn_memory_percent,
                "critical_memory_percent": resources.config.critical_memory_percent,
                "warning_swap_percent": resources.config.warn_swap_percent,
                "max_agents": resources.config.max_agents,
            },
        },
    }
