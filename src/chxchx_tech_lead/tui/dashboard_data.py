from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..core.models import ProjectInfo
from ..core.registry import load_registry
from ..workspace.agent_status import AgentRuntimeStatus
from ..workspace.manager import WorkspaceInspection
from ..workspace.process_manager import ProcessManager
from ..workspace.resources import (
    ProcessResources,
    ResourceManager,
    SystemResources,
    summarize_process_resources,
)
from ..workspace.service import WorkspaceOperationError, WorkspaceService

AggregatedResourceRow = tuple[str, str, str, str, str, int, float | None]


@dataclass(frozen=True, slots=True)
class DashboardSnapshot:
    inspection: WorkspaceInspection
    agent_statuses: list[AgentRuntimeStatus] | None
    agent_status_error: str | None
    system: SystemResources | None
    processes: list
    process_metrics: list[ProcessResources]
    aggregated_resources: list[AggregatedResourceRow] | None


def collect_dashboard(
    project: ProjectInfo,
    service: WorkspaceService,
    *,
    include_aggregated_resources: bool,
) -> DashboardSnapshot:
    """Read workspace, agent and resource state away from the UI event loop."""
    inspection = service.inspect()
    statuses = None
    status_error = None
    try:
        statuses = service.agent_statuses(probe_versions=False)
    except Exception as exc:
        status_error = str(exc)

    aggregated = (
        collect_aggregated_resources(project)
        if include_aggregated_resources
        else None
    )
    if inspection.config is None:
        return DashboardSnapshot(inspection, statuses, status_error, None, [], [], aggregated)

    resources = ResourceManager(inspection.config.resources)
    system = resources.system()
    processes = ProcessManager(
        project.root,
        inspection.config.processes,
        trusted=inspection.trusted,
    ).list()
    process_metrics = resources.processes(processes)
    return DashboardSnapshot(
        inspection,
        statuses,
        status_error,
        system,
        processes,
        process_metrics,
        aggregated,
    )


def collect_aggregated_resources(project: ProjectInfo) -> list[AggregatedResourceRow]:
    registry = load_registry()
    entries = list(registry.get("projects", []))
    current_root = project.root.resolve()
    known_paths = {
        str(Path(str(item.get("path", ""))).expanduser().resolve())
        for item in entries
        if item.get("path")
    }
    if str(current_root) not in known_paths:
        entries.append({"alias": project.name, "name": project.name, "path": str(current_root)})

    rows: list[AggregatedResourceRow] = []
    for item in entries:
        raw_path = item.get("path")
        if not isinstance(raw_path, str) or not raw_path.strip():
            continue
        root = Path(raw_path).expanduser()
        alias = str(item.get("alias", root.name))
        name = str(item.get("name", root.name))
        if not root.is_dir():
            rows.append(("*" if root.resolve() == current_root else "", alias, name, "NO EXISTE", "-", 0, None))
            continue

        try:
            root = root.resolve()
            other_project = ProjectInfo(root=root, name=name)
            inspection = WorkspaceService(other_project).inspect()
            if inspection.config is None:
                rows.append(("*" if root == current_root else "", alias, name, "ERROR CONFIG", "-", 0, None))
                continue
            managed = ProcessManager(
                root,
                inspection.config.processes,
                trusted=inspection.trusted,
            ).list(persist=False)
            manager = ResourceManager(inspection.config.resources)
            usage = summarize_process_resources(manager.processes(managed))
            rows.append(
                (
                    "*" if root == current_root else "",
                    alias,
                    name,
                    inspection.state.status.value,
                    str(usage.running_count),
                    usage.rss_bytes,
                    usage.cpu_percent,
                )
            )
        except (WorkspaceOperationError, OSError, ValueError):
            rows.append(("*" if root.resolve() == current_root else "", alias, name, "ERROR", "-", 0, None))

    return sorted(rows, key=lambda row: (row[0] != "*", row[1].casefold()))
