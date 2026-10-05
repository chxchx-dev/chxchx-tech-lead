from __future__ import annotations

from pathlib import Path
from typing import Any

from ..core.detector import detect_project
from ..core.registry import load_registry
from .manager import WorkspaceManager
from .process_manager import ManagedProcess, ProcessManager, ProcessStatus
from .resources import ResourceManager, summarize_process_resources


def resources_overview_payload(current_path: Path) -> dict[str, Any]:
    """Return one system snapshot and read-only managed-process totals per registered project."""
    registry = load_registry()
    entries = registry.get("projects", [])
    paths: dict[str, tuple[str, Path]] = {}
    for entry in entries:
        raw_path = entry.get("path")
        if isinstance(raw_path, str):
            path = Path(raw_path).expanduser()
            if path.is_dir():
                path = path.resolve()
                paths[str(path)] = (str(entry.get("alias", path.name)), path)
    current = current_path.expanduser().resolve()
    paths.setdefault(str(current), (current.name, current))

    projects: list[dict[str, Any]] = []
    system_snapshot = None
    for key, (alias, path) in sorted(paths.items()):
        project = detect_project(path)
        inspection = WorkspaceManager(project).inspect()
        config = inspection.config
        resources = ResourceManager(config.resources if config is not None else None)
        if system_snapshot is None:
            system_snapshot = resources.system()
        manager = ProcessManager(
            project.root,
            config.processes if config is not None else (),
            trusted=inspection.trusted,
        )
        process_map = {item.id: item for item in manager.list(persist=False)}
        for configured in config.processes if config is not None else ():
            process_map.setdefault(
                configured.id,
                ManagedProcess(
                    id=configured.id,
                    label=configured.label,
                    command=configured.command,
                    cwd=(project.root / configured.cwd).resolve(),
                    status=ProcessStatus.STOPPED,
                    port=configured.port,
                ),
            )
        measured = resources.processes(sorted(process_map.values(), key=lambda item: item.id))
        summary = summarize_process_resources(measured)
        projects.append({
            "alias": alias,
            "name": project.name,
            "path": key,
            "status": inspection.state.status.value,
            "trusted": inspection.trusted,
            "running_count": summary.running_count,
            "rss_bytes": summary.rss_bytes,
            "cpu_percent": summary.cpu_percent,
            "warning_memory_percent": resources.config.warn_memory_percent,
            "critical_memory_percent": resources.config.critical_memory_percent,
            "max_agents": resources.config.max_agents,
        })

    if system_snapshot is None:
        system_snapshot = ResourceManager().system()
    return {
        "schema": "chxchx.resources-overview",
        "schema_version": 1,
        "system": {
            "memory_total_bytes": system_snapshot.total_bytes,
            "memory_used_bytes": system_snapshot.used_bytes,
            "memory_available_bytes": system_snapshot.available_bytes,
            "memory_percent": system_snapshot.memory_percent,
            "swap_total_bytes": system_snapshot.swap_total_bytes,
            "swap_used_bytes": system_snapshot.swap_used_bytes,
            "swap_percent": system_snapshot.swap_percent,
            "cpu_percent": system_snapshot.cpu_percent,
        },
        "projects": projects,
    }
