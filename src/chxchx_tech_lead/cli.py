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

app = typer.Typer(no_args_is_help=True, help="Bootstrapper personal para tu entorno de desarrollo asistido por agentes.")
projects_app = typer.Typer(no_args_is_help=True, help="Administra proyectos registrados.")
workspace_app = typer.Typer(no_args_is_help=True, help="Inspecciona y administra el workspace del proyecto.")
process_app = typer.Typer(no_args_is_help=True, help="Administra procesos configurados del workspace.")
app.add_typer(projects_app, name="projects")
app.add_typer(workspace_app, name="workspace")
app.add_typer(process_app, name="process")
console = Console()


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


@app.command("run")
def run_workspace(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la receta sin ejecutarla."),
    attach: bool = typer.Option(True, "--attach/--no-attach", help="Entra en Zellij al finalizar."),
    recreate: bool = typer.Option(False, "--recreate", help="Recrea la sesión Zellij para aplicar el layout actual."),
):
    """Arranca workspace, agentes configurados y adjunta Zellij."""
    try:
        action, agents, attached = _workspace_service(path).run_all(
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
    """Detiene procesos y Docker gestionados del proyecto."""
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


@app.command("install")
def install_command(
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra acciones sin ejecutarlas."),
):
    """Instala las herramientas base administradas por uv."""
    for tool in ("basic-memory", "serena"):
        console.print(f"[cyan]→[/] {tool}")
        try:
            result = install_tool(tool, dry_run=dry_run)
        except RuntimeError as exc:
            console.print(f"[red]✗[/] {exc}")
            raise typer.Exit(code=1)
        if result.returncode == 0:
            if result.skipped:
                console.print(f"[green]✓[/] {tool} ya está instalado")
            else:
                console.print(f"[green]✓[/] {'DRY RUN ' if dry_run else ''}{' '.join(result.command)}")
        else:
            console.print(f"[red]✗[/] {' '.join(result.command)}")
            if result.stderr:
                console.print(result.stderr)


@app.command("init")
def init_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra cambios sin escribirlos."),
    no_backup: bool = typer.Option(False, "--no-backup", help="No crea backup antes de modificar archivos administrados."),
    minimal: bool = typer.Option(
        False,
        "--minimal",
        help="Deja solo .ai/chxchx-tech.toml y .ai/memory en el proyecto.",
    ),
):
    """Inicializa el proyecto; usa --minimal para una huella compacta."""
    info = _project(path)
    _show_project(info)

    planned_structure = create_project_structure(info, dry_run=True, minimal=minimal)
    planned_config = project_config_needs_update(info)
    planned_agents = False
    planned_claude = False
    planned_opencode = False
    if not minimal:
        planned_agents = upsert_managed_block(
            info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=True
        )
        planned_claude = upsert_managed_block(
            info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=True
        )
        planned_opencode = _opencode_example_needs_update(info)

    if not dry_run and not no_backup and any(
        (planned_structure, planned_config, planned_agents, planned_claude, planned_opencode)
    ):
        backup = backup_project(info.root)
        if backup:
            console.print(f"[dim]Backup: {backup}[/]")

    actions = create_project_structure(info, dry_run=dry_run, minimal=minimal)
    planned_memory = not (info.root / ".ai" / "memory").exists()
    ensure_project_config(info, dry_run=dry_run)
    if planned_config or planned_memory:
        actions.append("ensure .ai/chxchx-tech.toml + .ai/memory")
    if basic_memory_available():
        memory_result = ensure_memory_project(info, dry_run=dry_run)
        if memory_result is not None:
            if memory_result.returncode != 0:
                console.print(f"[yellow]! Basic Memory project:[/] {memory_result.stderr or memory_result.stdout}")
            elif not memory_result.skipped:
                actions.append("ensure Basic Memory project")
    else:
        console.print("[yellow]! Basic Memory no está instalado; ejecuta `chxchx-tech install`.[/]")

    if not minimal:
        if upsert_managed_block(info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=dry_run):
            actions.append("update AGENTS.md")
        if upsert_managed_block(info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=dry_run):
            actions.append("update CLAUDE.md")
    if register_project(info, dry_run=dry_run):
        actions.append("register project")
    if not minimal and write_opencode_example(info, dry_run=dry_run):
        actions.append("ensure OpenCode MCP example")

    if actions:
        for action in actions:
            console.print(f"[green]✓[/] {action}")
    else:
        console.print("[green]✓ Todo está actualizado.[/]")

    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")


@app.command()
def setup(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza instalación, init e integraciones."),
    minimal: bool = typer.Option(False, "--minimal", help="Usa la preparación compacta del proyecto."),
):
    """Flujo completo: herramientas base + init del proyecto + MCP."""
    console.print("[bold cyan]1/3 Herramientas base[/]")
    install_command(dry_run=dry_run)
    console.print("\n[bold cyan]2/3 Inicialización del proyecto[/]")
    init_command(path=path, dry_run=dry_run, no_backup=False, minimal=minimal)
    console.print("\n[bold cyan]3/3 Integraciones MCP[/]")
    integrate(path=path, client="all", dry_run=dry_run)



@app.command()
def doctor(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Diagnostica herramientas globales y configuración del proyecto."""
    table = Table(title="ChxChx Tech Lead — Doctor")
    table.add_column("Componente")
    table.add_column("Estado")
    table.add_column("Comando")
    for item in check_tools():
        table.add_row(item.name, "✓" if item.installed else "✗", item.command)
    console.print(table)

    info = _project(path)
    console.print()
    _show_project(info)
    checks = {
        "AGENTS.md": info.root / "AGENTS.md",
        "CLAUDE.md": info.root / "CLAUDE.md",
        ".ai": info.root / ".ai",
        ".ai/chxchx-tech.toml": info.root / ".ai" / "chxchx-tech.toml",
        ".ai/memory": info.root / ".ai" / "memory",
        ".ai/integrations/opencode-mcp.example.json": info.root / ".ai" / "integrations" / "opencode-mcp.example.json",
        "docs/adr": info.root / "docs" / "adr",
    }
    for label, item in checks.items():
        mark = "[green]✓[/]" if item.exists() else "[yellow]![/]"
        console.print(f"{mark} {label}")
    if (info.root / ".ai" / "chxchx-tech.toml").exists() and project_config_needs_update(info):
        console.print("[yellow]! .ai/chxchx-tech.toml está desactualizado; ejecuta `chxchx-tech init`.[/]")

    console.print("\n[bold]WORKSPACE[/]")
    inspection = WorkspaceManager(info).inspect()
    config_path = info.root / ".ai" / "chxchx-tech.toml"
    if not config_path.exists():
        console.print("[yellow]! .ai/chxchx-tech.toml no existe; ejecuta `chxchx-tech init`.[/]")
    elif inspection.error:
        console.print(f"[red]✗ Configuración workspace:[/] {inspection.error}")
    else:
        console.print(f"[green]✓[/] .ai/chxchx-tech.toml válido")
        trust_mark = "[green]✓[/]" if inspection.trusted else "[yellow]![/]"
        trust_text = "confiable" if inspection.trusted else "no confiable; ejecuta `chxchx-tech workspace trust`"
        console.print(f"{trust_mark} trust: {trust_text}")
        if inspection.config is not None:
            for item in (*inspection.config.processes, *inspection.config.agents):
                cwd = (info.root / item.cwd).resolve()
                mark = "[green]✓[/]" if cwd.is_dir() else "[red]✗[/]"
                console.print(f"{mark} {item.id} cwd: {cwd}")
        for warning in inspection.warnings:
            console.print(f"[yellow]! {warning}[/]")

    console.print("\n[bold]MCP POR PROYECTO[/]")
    diagnostics = Table(title="Basic Memory y Serena · configuración (solo lectura)")
    diagnostics.add_column("Cliente")
    diagnostics.add_column("Servidor")
    diagnostics.add_column("Estado")
    diagnostics.add_column("Alcance")
    diagnostics.add_column("Detalle")
    for item in diagnose_project_mcp(info):
        style = {"OK": "green", "AVISO": "yellow", "FALTA": "yellow", "ERROR": "red"}.get(
            item.status, "white"
        )
        diagnostics.add_row(
            item.client,
            item.server,
            f"[{style}]{item.status}[/{style}]",
            item.scope,
            item.detail,
        )
    console.print(diagnostics)
    console.print(
        "[dim]Solo lectura: no inicia agentes/servidores ni imprime variables de entorno. "
        "Codex debe confiar en el proyecto para cargar su configuración local.[/]"
    )


@app.command()
def status(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Resume el proyecto actual."""
    info = _project(path)
    _show_project(info)
    console.print(f"[bold]AI init:[/] {'sí' if (info.root / '.ai').exists() else 'no'}")
    console.print(f"[bold]AGENTS.md:[/] {'sí' if (info.root / 'AGENTS.md').exists() else 'no'}")
    console.print(f"[bold]CLAUDE.md:[/] {'sí' if (info.root / 'CLAUDE.md').exists() else 'no'}")


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


@workspace_app.command("status")
def workspace_status(
    path: str = typer.Argument("."),
):
    """Muestra configuración, confianza y estado operativo sin ejecutar procesos."""
    info, inspection = _workspace_inspection(path)
    console.print(f"[bold]Proyecto:[/] {info.name}")
    console.print(f"[bold]Config:[/] {inspection.config_path}")
    console.print(f"[bold]Trust:[/] {'sí' if inspection.trusted else 'no'}")
    console.print(f"[bold]Workspace:[/] {inspection.state.status.value}")
    if inspection.config is not None:
        console.print(f"[bold]Procesos configurados:[/] {len(inspection.config.processes)}")
        _print_agent_statuses(_workspace_service(path).agent_statuses())
    for warning in inspection.warnings:
        console.print(f"[yellow]! {warning}[/]")


@workspace_app.command("trust")
def workspace_trust(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el cambio sin guardarlo."),
):
    """Marca localmente un proyecto como confiable para poder ejecutar procesos."""
    config_path = path / ".ai" / "chxchx-tech.toml"
    if not config_path.exists():
        console.print("[red]✗ Falta .ai/chxchx-tech.toml; ejecuta `chxchx-tech init` primero.[/]")
        raise typer.Exit(code=1)
    changed = trust_project(path, dry_run=dry_run)
    if changed:
        console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}proyecto confiable: {path.resolve()}")
    else:
        console.print("[green]✓ El proyecto ya estaba confiable.[/]")
    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")


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


@workspace_app.command("open")
def workspace_open(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra las acciones sin ejecutarlas."),
    attach: bool = typer.Option(False, "--attach", help="Adjunta la terminal después de preparar la sesión."),
):
    """Prepara una sesión y aplica las acciones auto_start configuradas."""
    try:
        action = _workspace_service(path).open(dry_run=dry_run, attach=attach)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)


@workspace_app.command("start")
def workspace_start(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra las acciones sin ejecutarlas."),
):
    """Inicia la sesión, procesos auto_start y Docker configurado."""
    try:
        action = _workspace_service(path).start(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)


@workspace_app.command("stop")
def workspace_stop(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra las acciones sin ejecutarlas."),
):
    """Detiene procesos administrados y Docker del proyecto."""
    try:
        action = _workspace_service(path).stop(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)


@workspace_app.command("suspend")
def workspace_suspend(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la suspensión sin ejecutarla."),
):
    """Detiene procesos administrados y conserva el workspace como recuperable."""
    try:
        action = _workspace_service(path).suspend(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)


@workspace_app.command("resume")
def workspace_resume(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la reanudación sin ejecutarla."),
):
    """Reanuda un workspace suspendido y sus procesos configurados."""
    try:
        action = _workspace_service(path).resume(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_workspace_action(action)


@workspace_app.command("attach")
def workspace_attach(
    path: str = typer.Argument("."),
):
    """Adjunta la terminal a la sesión existente del workspace."""
    try:
        _workspace_service(path).attach()
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)


editor_app = typer.Typer(no_args_is_help=True, help="Administra el editor configurado.")
agent_app = typer.Typer(no_args_is_help=True, help="Administra agentes configurados.")
app.add_typer(editor_app, name="editor")
app.add_typer(agent_app, name="agent")


@editor_app.command("open")
def editor_open(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el comando sin ejecutarlo."),
):
    """Abre el proyecto en Sublime mediante su adapter."""
    try:
        result = _workspace_service(path).open_editor(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")


@agent_app.command("start")
def agent_start(
    agent_id: str | None = typer.Argument(None, metavar="ID"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el inicio sin ejecutarlo."),
    all_agents: bool = typer.Option(False, "--all", help="Inicia todos los agentes configurados."),
    preset: str | None = typer.Option(None, "--preset", help="Inicia el preset declarado en el proyecto."),
):
    """Inicia un agente, o todos los agentes configurados, dentro del workspace."""
    selected = sum(value is not None for value in (agent_id, preset)) + int(all_agents)
    if selected > 1:
        console.print("[red]✗ Usa un ID, `--all` o `--preset`, no varios.[/]")
        raise typer.Exit(code=2)
    if selected == 0:
        console.print("[red]✗ Indica un ID, usa `--all` o `--preset`.[/]")
        raise typer.Exit(code=2)
    try:
        service = _workspace_service(path)
        if preset is not None:
            results = service.start_preset(preset, dry_run=dry_run)
        elif all_agents:
            results = service.start_agents(dry_run=dry_run)
        else:
            result = service.start_agent(agent_id, dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    if preset is not None or all_agents:
        for current_id, current_result in results:
            console.print(f"[green]✓[/] {current_id}: {'DRY RUN: ' if dry_run else ''}{' '.join(current_result.command)}")
    else:
        console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")


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


@agent_app.command("list")
def agent_list(
    path: str = typer.Option(".", "--path"),
):
    """Lista disponibilidad y estado de los agentes sin iniciarlos."""
    try:
        _print_agent_statuses(_workspace_service(path).agent_statuses())
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)


@agent_app.command("status")
def agent_status(
    path: str = typer.Option(".", "--path"),
):
    """Alias explícito de `agent list`."""
    agent_list(path=path)


@workspace_app.command("handoff")
def workspace_handoff(
    path: str = typer.Argument("."),
    summary: str = typer.Option("Estado del workspace actualizado.", "--summary"),
    pending: str = typer.Option("Revisar el siguiente entregable.", "--pending"),
    validation: str = typer.Option("Ejecutar las pruebas relevantes antes de continuar.", "--validation"),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el cambio sin escribirlo."),
):
    """Actualiza el bloque administrado del handoff del proyecto."""
    try:
        service = _workspace_service(path)
        inspection = service.inspect()
        changed = update_handoff(
            service.project,
            inspection.state.status,
            service.agent_statuses(),
            summary=summary,
            pending=pending,
            validation=validation,
            dry_run=dry_run,
        )
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{'handoff actualizado' if changed else 'handoff sin cambios'}")


def _process_manager(path: Path):
    _info, inspection = _workspace_inspection(path)
    if inspection.config is None:
        raise typer.Exit(code=1)
    return ProcessManager(path, inspection.config.processes, trusted=inspection.trusted)


@process_app.command("list")
def process_list(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Lista procesos configurados y su estado actual."""
    manager = _process_manager(path)
    records = manager.list()
    if not records:
        console.print("[yellow]No hay procesos configurados.[/]")
        return
    table = Table(title=f"Procesos — {path.resolve().name}")
    table.add_column("ID")
    table.add_column("Estado")
    table.add_column("PID")
    table.add_column("Puerto")
    table.add_column("Comando")
    for record in records:
        command = record.command if isinstance(record.command, str) else " ".join(record.command)
        table.add_row(record.id, record.status.value, str(record.pid or "-"), str(record.port or "-"), command)
    console.print(table)


@process_app.command("start")
def process_start(
    process_id: str = typer.Argument(..., metavar="ID"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el inicio sin ejecutarlo."),
):
    """Inicia un proceso configurado después de validar trust y cwd."""
    try:
        result = _process_manager(path).start(process_id, dry_run=dry_run)
    except ProcessManagerError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {result.message}")


@process_app.command("stop")
def process_stop(
    process_id: str = typer.Argument(..., metavar="ID"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la parada sin ejecutarla."),
):
    """Detiene un proceso administrado verificando su PID."""
    try:
        result = _process_manager(path).stop(process_id, dry_run=dry_run)
    except ProcessManagerError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {result.message}")


@app.command("resources")
def resources(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Muestra RAM, swap, CPU y consumo de procesos administrados."""
    _info, inspection = _workspace_inspection(path)
    config = inspection.config.resources if inspection.config is not None else None
    manager = ResourceManager(config)
    system = manager.system()
    severity = manager.severity(system)
    console.print(f"[bold]Resource Manager — {path.resolve().name}[/]")
    console.print(
        f"RAM       {format_bytes(system.used_bytes)} / {format_bytes(system.total_bytes)} "
        f"({system.memory_percent:.0f}%)"
    )
    console.print(
        f"Swap      {format_bytes(system.swap_used_bytes)} / {format_bytes(system.swap_total_bytes)} "
        f"({system.swap_percent:.0f}%)"
    )
    cpu = "N/D" if system.cpu_percent is None else f"{system.cpu_percent:.0f}%"
    console.print(f"CPU       {cpu}")
    mark = "[green]" if severity is ResourceSeverity.OK else "[yellow]" if severity is ResourceSeverity.WARNING else "[red]"
    console.print(f"Estado    {mark}{severity.value}[/]")

    process_manager = ProcessManager(
        path,
        inspection.config.processes if inspection.config is not None else (),
        trusted=inspection.trusted,
    )
    process_metrics = manager.processes(process_manager.list())
    if process_metrics:
        table = Table(title="Procesos administrados")
        table.add_column("ID")
        table.add_column("Estado")
        table.add_column("PID")
        table.add_column("RAM")
        table.add_column("CPU")
        for metric in process_metrics:
            cpu_value = "N/D" if metric.cpu_percent is None else f"{metric.cpu_percent:.1f}%"
            table.add_row(
                metric.process_id,
                metric.status.value,
                str(metric.pid or "-"),
                format_bytes(metric.rss_bytes),
                cpu_value,
            )
        console.print(table)


@app.command()
def sync(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run"),
    no_backup: bool = typer.Option(False, "--no-backup", help="No crea backup antes de sincronizar."),
):
    """Sincroniza los bloques administrados por ChxChx."""
    result = sync_project(path, dry_run=dry_run, create_backup=not no_backup)
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
    console.print("[green]✓ sincronización aplicada[/]" if result.changed else "[green]✓ sin cambios[/]")
    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")


@projects_app.command("list")
def projects_list():
    """Lista los proyectos registrados en el estado global."""
    data = load_registry()
    projects = data.get("projects", [])
    if not projects:
        console.print("[yellow]No hay proyectos registrados.[/]")
        return

    table = Table(title="Proyectos registrados")
    table.add_column("Alias")
    table.add_column("Nombre")
    table.add_column("Perfil")
    table.add_column("Estado")
    table.add_column("Ruta")
    for project in projects:
        raw_path = project.get("path")
        project_path = Path(raw_path) if raw_path else Path("<ruta ausente>")
        state = load_state(project_path).status.value if raw_path and project_path.is_dir() else "no encontrado"
        if raw_path and project_path.is_dir() and data.get("last_project") == str(project_path.resolve()):
            state += " · último"
        table.add_row(
            str(project.get("alias", project.get("name", ""))),
            str(project.get("name", "")),
            str(project.get("profile", "generic")),
            state,
            str(project_path),
        )
    console.print(table)


@projects_app.command("current")
def projects_current():
    """Muestra el último proyecto activado, si sigue disponible."""
    data = load_registry()
    raw_path = data.get("last_project")
    if not isinstance(raw_path, str) or not Path(raw_path).is_dir():
        console.print("[yellow]No hay un último proyecto disponible.[/]")
        return
    project = next(
        (item for item in data.get("projects", []) if item.get("path") == raw_path),
        None,
    )
    alias = project.get("alias") if isinstance(project, dict) else Path(raw_path).name
    console.print(f"[green]✓[/] {alias}: {Path(raw_path).resolve()}")


@projects_app.command("switch")
def projects_switch(
    alias: str = typer.Argument(..., metavar="ALIAS"),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la transición sin ejecutarla."),
    attach: bool = typer.Option(True, "--attach/--no-attach", help="Adjunta el workspace después del cambio."),
    recreate: bool = typer.Option(False, "--recreate", help="Recrea la sesión del proyecto destino."),
):
    """Suspende el workspace activo y activa otro proyecto registrado."""
    try:
        target = _resolve_project_path(alias)
        target_info = _project(target)
        for project in load_registry().get("projects", []):
            raw_path = project.get("path")
            if not isinstance(raw_path, str) or Path(raw_path).resolve() == target:
                continue
            other = Path(raw_path)
            if not other.is_dir() or load_state(other).status.value != "ACTIVE":
                continue
            WorkspaceService(_project(other)).suspend(dry_run=dry_run)
            console.print(f"[green]✓[/] workspace suspendido: {other}")
        service = WorkspaceService(target_info)
        action, agents, attached = service.run_all(
            dry_run=dry_run, attach=attach, recreate=recreate
        )
    except (WorkspaceOperationError, ValueError) as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    _print_recipe(action, agents, attached)
    console.print(f"[green]✓[/] proyecto activo: {target_info.name} ({alias})")


@projects_app.command("sync")
def projects_sync(
    dry_run: bool = typer.Option(False, "--dry-run"),
    no_backup: bool = typer.Option(False, "--no-backup", help="No crea backups al sincronizar."),
):
    """Sincroniza los bloques administrados de todos los proyectos registrados."""
    projects = load_registry().get("projects", [])
    if not projects:
        console.print("[yellow]No hay proyectos registrados.[/]")
        return

    for project in projects:
        raw_path = project.get("path")
        if not raw_path:
            console.print("[yellow]! Omitido, registro sin ruta.[/]")
            continue
        project_path = Path(raw_path)
        if not project_path.is_dir():
            console.print(f"[yellow]! Omitido, no existe:[/] {project_path}")
            continue
        result = sync_project(project_path, dry_run=dry_run, create_backup=not no_backup)
        mark = "sin cambios" if not result.changed else "sincronizado"
        console.print(f"[green]✓[/] {project_path}: {mark}")
    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")


@app.command()
def integrate(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    client: str = typer.Option("all", "--client", help="all, claude, codex u opencode"),
    dry_run: bool = typer.Option(False, "--dry-run"),
    refresh: bool = typer.Option(False, "--refresh", help="Reemplaza servidores MCP existentes con la configuración actual."),
):
    """Configura integraciones MCP en clientes soportados."""
    info = _project(path)
    clients = ["claude", "codex", "opencode"] if client == "all" else [client]
    for current in clients:
        if current not in {"claude", "codex", "opencode"}:
            console.print(f"[red]Cliente inválido:[/] {current}")
            raise typer.Exit(code=2)
        try:
            results = integrate_mcp(info, current, dry_run=dry_run, refresh=refresh)
        except RuntimeError as exc:
            console.print(f"[yellow]![/] {exc}")
            continue
        for result in results:
            mark = "[green]✓[/]" if result.returncode == 0 else "[red]✗[/]"
            console.print(f"{mark} {' '.join(result.command)}")
            if result.returncode != 0 and result.stderr:
                console.print(f"[dim]{result.stderr}[/]")


@app.command()
def rollback(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Restaura el último backup de archivos administrados del proyecto."""
    root = path.resolve()
    restore_target = latest_backup(root)
    if restore_target is None:
        console.print("[yellow]No hay backups para este proyecto.[/]")
        raise typer.Exit(code=1)
    safety_backup = backup_project(root)
    restored = restore_backup(root, restore_target)
    if safety_backup:
        console.print(f"[dim]Backup del estado actual: {safety_backup}[/]")
    console.print(f"[green]✓ Restaurado desde:[/] {restored}")


def _opencode_example_needs_update(info) -> bool:
    target = info.root / ".ai" / "integrations" / "opencode-mcp.example.json"
    if not target.exists():
        return True
    return write_opencode_example(info, dry_run=True)


if __name__ == "__main__":
    app()
