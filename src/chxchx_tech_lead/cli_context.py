from __future__ import annotations

from pathlib import Path

import typer
from rich.console import Console
from rich.table import Table

from . import __version__
from .core.backup import backup_project, latest_backup, restore_backup
from .core.detector import detect_project
from .core.managed import upsert_managed_block
from .core.registry import load_registry, register_project, resolve_project_reference
from .core.project_config import ensure_project_config, project_config_needs_update
from .core.sync import sync_project
from .core.templates import claude_body, create_project_structure, project_rule_body
from .core.trust import trust_project
from .integrations.installers import install_tool
from .integrations.basic_memory import ensure_project as ensure_memory_project, available as basic_memory_available
from .integrations.mcp import integrate as integrate_mcp, write_opencode_example
from .integrations.mcp_diagnostics import diagnose_project_mcp
from .integrations.tools import check_tools
from .workspace.manager import WorkspaceManager
from .workspace.state import load_state
from .workspace.handoff import update_handoff
from .workspace.process_manager import ProcessManager, ProcessManagerError, ProcessStatus
from .workspace.resources import ResourceManager, ResourceSeverity, format_bytes
from .workspace.service import WorkspaceOperationError, WorkspaceService
from .tui import TUIUnavailableError, run_tui
from .ui.theme import cli_theme




app = typer.Typer(no_args_is_help=True, help="Bootstrapper personal para tu entorno de desarrollo asistido por agentes.")
projects_app = typer.Typer(no_args_is_help=True, help="Administra proyectos registrados.")
workspace_app = typer.Typer(no_args_is_help=True, help="Inspecciona y administra el workspace del proyecto.")
process_app = typer.Typer(no_args_is_help=True, help="Administra procesos configurados del workspace.")
editor_app = typer.Typer(no_args_is_help=True, help="Administra el editor configurado.")
agent_app = typer.Typer(no_args_is_help=True, help="Administra agentes configurados.")
skill_app = typer.Typer(no_args_is_help=True, help="Busca y recomienda skills del proyecto.")
pack_app = typer.Typer(no_args_is_help=True, help="Detecta y aplica Tech Packs de skills.")
bridge_app = typer.Typer(no_args_is_help=True, help="API JSON local para interfaces de ChxChx.")
app.add_typer(projects_app, name="projects")
app.add_typer(workspace_app, name="workspace")
app.add_typer(process_app, name="process")
app.add_typer(editor_app, name="editor")
app.add_typer(agent_app, name="agent")
app.add_typer(skill_app, name="skill")
app.add_typer(pack_app, name="pack")
app.add_typer(bridge_app, name="bridge")
console = Console(theme=cli_theme())



def _project(path: Path):
    return detect_project(path)

def _show_project(info):
    console.print(f"[bold]Proyecto:[/] {info.name}")
    console.print(f"[bold]Perfil:[/] {info.profile_name}")
    console.print(f"[bold]Stacks:[/] {', '.join(info.stacks)}")
    console.print(f"[bold]Lenguajes:[/] {', '.join(info.languages) or 'no detectados'}")

def _print_recipe(action, agents, attached) -> None:
    _print_workspace_action(action)
    if action.session_created and action.inspection.config and action.inspection.config.agents:
        console.print(
            f"[green]✓[/] agentes en layout: {', '.join(agent.id for agent in action.inspection.config.agents)}"
        )
    for agent_id, result in agents:
        console.print(f"[green]✓[/] agente {agent_id}: {' '.join(result.command)}")
    if attached is not None:
        console.print("[green]✓[/] sesión adjuntada")

def _workspace_inspection(path: str | Path):
    try:
        info = _project(_resolve_project_path(path))
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    inspection = WorkspaceManager(info).inspect()
    if inspection.error:
        console.print(f"[red]✗ Configuración inválida:[/] {inspection.error}")
        raise typer.Exit(code=1)
    return info, inspection

def _resolve_project_path(reference: str | Path) -> Path:
    try:
        return resolve_project_reference(reference)
    except ValueError as exc:
        raise WorkspaceOperationError(str(exc)) from exc

def _workspace_service(path: str | Path) -> WorkspaceService:
    return WorkspaceService(_project(_resolve_project_path(path)))

def _print_workspace_action(action) -> None:
    for message in action.messages:
        console.print(f"[green]✓[/] {message}")
    for result in action.process_results:
        console.print(f"[green]✓[/] {result.message}")

def _print_agent_statuses(statuses) -> None:
    if not statuses:
        console.print("[yellow]No hay agentes configurados.[/]")
        return
    table = Table(title="Agentes")
    table.add_column("ID")
    table.add_column("Disponible")
    table.add_column("Sesión")
    table.add_column("Pane")
    table.add_column("Preset")
    table.add_column("Versión")
    for agent in statuses:
        table.add_row(
            agent.id,
            "✓" if agent.available else "✗",
            agent.session,
            agent.pane,
            agent.preset,
            (agent.version or "-").strip().splitlines()[0][:60],
        )
    console.print(table)

def _process_manager(path: Path):
    _info, inspection = _workspace_inspection(path)
    if inspection.config is None:
        raise typer.Exit(code=1)
    return ProcessManager(path, inspection.config.processes, trusted=inspection.trusted)

def _opencode_example_needs_update(info) -> bool:
    target = info.root / ".ai" / "integrations" / "opencode-mcp.example.json"
    if not target.exists():
        return True
    return write_opencode_example(info, dry_run=True)
