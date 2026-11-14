from __future__ import annotations

from pathlib import Path

import typer
from rich.table import Table

from ..cli_registry import process_app
from ..ui.cli_output import console
from ..workspace.process_manager import ProcessManagerError
from .workspace_context import process_manager_for as _process_manager

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
