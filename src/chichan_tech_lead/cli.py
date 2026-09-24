from __future__ import annotations

from pathlib import Path

import typer
from rich.console import Console
from rich.table import Table

from . import __version__
from .core.backup import backup_project, latest_backup, restore_backup
from .core.detector import detect_project
from .core.managed import upsert_managed_block
from .core.registry import load_registry, register_project
from .core.project_config import ensure_project_config, project_config_needs_update
from .core.sync import sync_project
from .core.templates import claude_body, create_project_structure, project_rule_body
from .integrations.installers import install_tool
from .integrations.basic_memory import ensure_project as ensure_memory_project, available as basic_memory_available
from .integrations.mcp import integrate as integrate_mcp, write_opencode_example
from .integrations.tools import check_tools

app = typer.Typer(no_args_is_help=True, help="Bootstrapper personal para tu entorno de desarrollo asistido por agentes.")
projects_app = typer.Typer(no_args_is_help=True, help="Administra proyectos registrados.")
app.add_typer(projects_app, name="projects")
console = Console()


def _project(path: Path):
    return detect_project(path)


def _show_project(info):
    console.print(f"[bold]Proyecto:[/] {info.name}")
    console.print(f"[bold]Perfil:[/] {info.profile_name}")
    console.print(f"[bold]Stacks:[/] {', '.join(info.stacks)}")
    console.print(f"[bold]Lenguajes:[/] {', '.join(info.languages) or 'no detectados'}")


@app.command()
def version():
    """Muestra la versión instalada."""
    console.print(f"chichan-tech-lead {__version__}")


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
):
    """Inicializa o actualiza la estructura AI del proyecto."""
    info = _project(path)
    _show_project(info)

    planned_structure = create_project_structure(info, dry_run=True)
    planned_config = project_config_needs_update(info)
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

    actions = create_project_structure(info, dry_run=dry_run)
    planned_memory = not (info.root / ".ai" / "memory").exists()
    ensure_project_config(info, dry_run=dry_run)
    if planned_config or planned_memory:
        actions.append("ensure .ai/chichan.toml + .ai/memory")
    if basic_memory_available():
        memory_result = ensure_memory_project(info, dry_run=dry_run)
        if memory_result is not None:
            if memory_result.returncode != 0:
                console.print(f"[yellow]! Basic Memory project:[/] {memory_result.stderr or memory_result.stdout}")
            elif not memory_result.skipped:
                actions.append("ensure Basic Memory project")
    else:
        console.print("[yellow]! Basic Memory no está instalado; ejecuta `chichan install`.[/]")

    if upsert_managed_block(info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=dry_run):
        actions.append("update AGENTS.md")
    if upsert_managed_block(info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=dry_run):
        actions.append("update CLAUDE.md")
    if register_project(info, dry_run=dry_run):
        actions.append("register project")
    if write_opencode_example(info, dry_run=dry_run):
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
):
    """Flujo completo: herramientas base + init del proyecto + MCP."""
    console.print("[bold cyan]1/3 Herramientas base[/]")
    install_command(dry_run=dry_run)
    console.print("\n[bold cyan]2/3 Inicialización del proyecto[/]")
    init_command(path=path, dry_run=dry_run, no_backup=False)
    console.print("\n[bold cyan]3/3 Integraciones MCP[/]")
    integrate(path=path, client="all", dry_run=dry_run)



@app.command()
def doctor(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Diagnostica herramientas globales y configuración del proyecto."""
    table = Table(title="Chichan Tech Lead — Doctor")
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
        ".ai/chichan.toml": info.root / ".ai" / "chichan.toml",
        ".ai/memory": info.root / ".ai" / "memory",
        ".ai/integrations/opencode-mcp.example.json": info.root / ".ai" / "integrations" / "opencode-mcp.example.json",
        "docs/adr": info.root / "docs" / "adr",
    }
    for label, item in checks.items():
        mark = "[green]✓[/]" if item.exists() else "[yellow]![/]"
        console.print(f"{mark} {label}")
    if (info.root / ".ai" / "chichan.toml").exists() and project_config_needs_update(info):
        console.print("[yellow]! .ai/chichan.toml está desactualizado; ejecuta `chichan init`.[/]")


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


@app.command()
def sync(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run"),
    no_backup: bool = typer.Option(False, "--no-backup", help="No crea backup antes de sincronizar."),
):
    """Sincroniza los bloques administrados por Chichan."""
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
    table.add_column("Nombre")
    table.add_column("Perfil")
    table.add_column("Estado")
    table.add_column("Ruta")
    for project in projects:
        raw_path = project.get("path")
        project_path = Path(raw_path) if raw_path else Path("<ruta ausente>")
        state = "disponible" if raw_path and project_path.is_dir() else "no encontrado"
        table.add_row(
            str(project.get("name", "")),
            str(project.get("profile", "generic")),
            state,
            str(project_path),
        )
    console.print(table)


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
):
    """Configura integraciones MCP en clientes soportados."""
    info = _project(path)
    clients = ["claude", "codex", "opencode"] if client == "all" else [client]
    for current in clients:
        if current not in {"claude", "codex", "opencode"}:
            console.print(f"[red]Cliente inválido:[/] {current}")
            raise typer.Exit(code=2)
        try:
            results = integrate_mcp(info, current, dry_run=dry_run)
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
