from __future__ import annotations

from ..cli_context import (
    Path,
    ProcessManager,
    ResourceManager,
    ResourceSeverity,
    Table,
    app,
    console,
    format_bytes,
    sync_project,
    typer,
    _workspace_inspection,
)

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
