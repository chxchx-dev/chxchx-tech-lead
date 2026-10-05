from __future__ import annotations

from ..cli_context import (
    editor_app,
    Path,
    WorkspaceOperationError,
    app,
    console,
    trust_project,
    typer,
    workspace_app,
    _print_agent_statuses,
    _print_workspace_action,
    _workspace_inspection,
    _workspace_service,
)
from ..workspace.error_cache import record_error

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
    """Marca localmente un proyecto como confiable para habilitar su workspace."""
    changed = trust_project(path, dry_run=dry_run)
    if changed:
        console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}proyecto confiable: {path.resolve()}")
    else:
        console.print("[green]✓ El proyecto ya estaba confiable.[/]")
    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")

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
    """Inicia la sesión y los procesos configurados para autoarranque."""
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
    """Detiene los procesos administrados del proyecto."""
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
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el attach sin abrir la terminal."),
):
    """Adjunta la terminal a la sesión existente del workspace."""
    try:
        result = _workspace_service(path).attach(dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")


@workspace_app.command("terminal")
def workspace_terminal(
    path: str = typer.Argument("."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la acción sin crear el pane."),
):
    """Abre otra terminal interactiva dentro de la sesión Zellij del proyecto."""
    service = None
    try:
        service = _workspace_service(path)
        console.print(f"[cyan]Preparando terminal paralela para {path}…[/]")
        result = service.open_terminal(dry_run=dry_run)
        if dry_run:
            console.print(f"[yellow]DRY RUN:[/] {' '.join(result.command)}")
            return
        console.print("[green]✓ Terminal abierta; al salir de Zellij volverás a esta consola.[/]")
    except (WorkspaceOperationError, OSError) as exc:
        console.print(f"[red]✗ {exc}[/]")
        if service is not None:
            try:
                record_error(
                    project=service.project.name,
                    project_path=service.project.root,
                    operation="workspace terminal",
                    message=f"{type(exc).__name__}: {exc}",
                )
            except OSError:
                pass
        raise typer.Exit(code=1)


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

@editor_app.command("setup")
def editor_setup(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la configuración y apertura sin escribir ni ejecutar."),
    open_editor: bool = typer.Option(True, "--open/--no-open", help="Abre el proyecto generado en Sublime."),
):
    """Genera la configuración local de Sublime con exclusiones útiles."""
    try:
        project_file, generated, opened = _workspace_service(path).setup_editor(
            dry_run=dry_run,
            open_after=open_editor,
        )
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {generated.stdout or project_file}")
    if opened is not None:
        console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(opened.command)}")
