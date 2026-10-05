"""CLI presentation helpers for workspace inspection and process commands."""

from __future__ import annotations

from pathlib import Path

import typer

from ..ui.cli_output import console
from ..workspace.manager import WorkspaceManager
from ..workspace.process_manager import ProcessManager
from ..workspace.project_context import project_for_path, resolve_project_path
from ..workspace.service import WorkspaceOperationError


def inspect_workspace(path: str | Path):
    """Resolve and inspect a project, rendering CLI errors consistently."""
    try:
        info = project_for_path(resolve_project_path(path))
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    inspection = WorkspaceManager(info).inspect()
    if inspection.error:
        console.print(f"[red]✗ Configuración inválida:[/] {inspection.error}")
        raise typer.Exit(code=1)
    return info, inspection


def process_manager_for(path: Path) -> ProcessManager:
    """Build the configured process adapter after workspace validation."""
    _info, inspection = inspect_workspace(path)
    if inspection.config is None:
        raise typer.Exit(code=1)
    return ProcessManager(path, inspection.config.processes, trusted=inspection.trusted)
