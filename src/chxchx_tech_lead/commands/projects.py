from __future__ import annotations

from ..cli_context import (
    Path,
    Table,
    WorkspaceOperationError,
    WorkspaceService,
    console,
    load_registry,
    load_state,
    projects_app,
    sync_project,
    typer,
    _print_recipe,
    _project,
    _resolve_project_path,
)

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
