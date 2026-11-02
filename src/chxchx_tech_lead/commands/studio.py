from __future__ import annotations

from pathlib import Path

import typer

from ..cli_registry import studio_app
from ..ui.cli_output import console
from ..workspace.project_context import workspace_service_for
from ..workspace.service import WorkspaceOperationError


@studio_app.command("open")
def studio_open(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el comando sin iniciar Studio."),
):
    """Abre ChxChx Studio con el proyecto indicado."""
    try:
        result = workspace_service_for(path).open_studio(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")
