from __future__ import annotations

from ..cli_context import (
    Path,
    app,
    backup_project,
    console,
    integrate_mcp,
    latest_backup,
    restore_backup,
    typer,
    _project,
)

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
