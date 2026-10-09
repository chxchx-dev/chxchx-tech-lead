from __future__ import annotations

from pathlib import Path

import typer

from .. import __version__
from ..ui.cli_output import console, print_recipe as _print_recipe, print_workspace_action as _print_workspace_action
from ..cli_registry import app
from ..tui import TUIUnavailableError, run_tui
from ..workspace.project_context import project_for_path as _project, workspace_service_for as _workspace_service
from ..workspace.service import WorkspaceOperationError
from .resource_guard import guard_cli_agent_start

@app.command("run")
def run_workspace(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la receta sin ejecutarla."),
    attach: bool = typer.Option(True, "--attach/--no-attach", help="Entra en Zellij al finalizar."),
    recreate: bool = typer.Option(False, "--recreate", help="Recrea la sesión Zellij para aplicar el layout actual."),
    force: bool = typer.Option(False, "--force", help="Confirma el inicio aunque exceda el presupuesto RAM/agentes."),
):
    """Arranca workspace, agentes configurados y adjunta Zellij."""
    try:
        service = _workspace_service(path)
        guard_cli_agent_start(service, dry_run=dry_run, force=force)
        action, agents, attached = service.run_all(
            dry_run=dry_run, attach=attach, recreate=recreate
        )
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_recipe(action, agents, attached)

@app.command("attach")
def attach_workspace(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Adjunta rápidamente la sesión Zellij del proyecto actual."""
    try:
        result = _workspace_service(path).attach()
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {' '.join(result.command)}")

@app.command("stop")
def stop_workspace(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la parada sin ejecutarla."),
):
    """Detiene los procesos gestionados del proyecto."""
    try:
        action = _workspace_service(path).stop(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)

@app.command()
def version():
    """Muestra la versión instalada."""
    console.print(f"chxchx-tech-lead {__version__}")

@app.command("tui")
def tui(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Abre el dashboard TUI cuando Textual está instalado."""
    try:
        run_tui(_project(path))
    except TUIUnavailableError as exc:
        console.print(f"[yellow]! {exc}[/]")
        raise typer.Exit(code=1)
