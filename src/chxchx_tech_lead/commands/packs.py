from __future__ import annotations

from ..cli_context import Path, Table, console, pack_app, typer
from ..skills import TechPack, TechPackRegistry, enable_skills, enabled_skills


def _show_packs(packs: list[TechPack]) -> None:
    if not packs:
        console.print("[yellow]No hay Tech Packs en la biblioteca.[/]")
        return
    table = Table(title="Tech Packs")
    table.add_column("Pack", style="cyan")
    table.add_column("Descripción")
    table.add_column("Skills", justify="right")
    for pack in packs:
        table.add_row(pack.name, pack.description, str(len(pack.skills)))
    console.print(table)
    for pack in packs:
        console.print(f"[dim]{pack.name}:[/] {', '.join(pack.skills)}")
    console.print(f"{len(packs)} packs · {sum(len(pack.skills) for pack in packs)} asignaciones de skills")


@pack_app.command("list")
def pack_list() -> None:
    """Lista los Tech Packs disponibles."""
    _show_packs(TechPackRegistry().list())


@pack_app.command("info")
def pack_info(name: str = typer.Argument(..., help="Nombre del Tech Pack.")) -> None:
    """Muestra las skills que compone un Tech Pack."""
    pack = TechPackRegistry().get(name)
    if pack is None:
        console.print(f"[red]Tech Pack desconocido:[/] {name}")
        raise typer.Exit(code=1)
    console.print(f"[bold cyan]{pack.name}[/] — {pack.description}")
    console.print("Skills: " + ", ".join(pack.skills))
    console.print("Combina cuando detecta: " + ", ".join((*pack.stacks, *pack.languages)))


@pack_app.command("detect")
def pack_detect(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Detecta Tech Packs compatibles y explica cada coincidencia."""
    matches = TechPackRegistry().detect(path)
    if not matches:
        console.print("[yellow]No hay Tech Packs compatibles con el stack detectado.[/]")
        return
    table = Table(title=f"Tech Packs recomendados para {path.name}")
    table.add_column("Pack", style="cyan")
    table.add_column("Motivo")
    for match in matches:
        table.add_row(match.pack.name, "; ".join(match.reasons))
    console.print(table)
    for match in matches:
        console.print(f"[dim]{match.pack.name}:[/] {', '.join(match.pack.skills)}")


@pack_app.command("apply")
def pack_apply(
    name: str = typer.Argument(..., help="Tech Pack a añadir al proyecto."),
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la selección sin escribirla."),
) -> None:
    """Añade las skills de un pack a la selección del proyecto."""
    registry = TechPackRegistry()
    pack = registry.get(name)
    if pack is None:
        console.print(f"[red]Tech Pack desconocido:[/] {name}")
        raise typer.Exit(code=1)
    current = set(enabled_skills(path))
    try:
        result = enable_skills(path, pack.skills, dry_run=dry_run)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    added = sorted(set(result.enabled) - current)
    if not result.changed:
        console.print(f"[green]✓[/] {pack.name} ya estaba aplicado; no hay cambios.")
        return
    status = "se añadirían" if dry_run else "añadidas"
    console.print(f"[green]✓[/] {pack.name}: {status} {', '.join(added)}")
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
    if not dry_run:
        console.print("Carga las instrucciones con: chxchx-tech skill sync .")

@pack_app.command("apply-detected")
def pack_apply_detected(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra la selección sin escribirla."),
) -> None:
    """Añade todos los packs compatibles sin pedir sus nombres."""
    registry = TechPackRegistry()
    matches = registry.detect(path)
    if not matches:
        console.print("[yellow]No se detectaron Tech Packs para aplicar.[/]")
        return
    names = tuple(dict.fromkeys(
        skill_name for match in matches for skill_name in match.pack.skills
    ))
    current = set(enabled_skills(path))
    try:
        result = enable_skills(path, names, dry_run=dry_run)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc

    added = sorted(set(result.enabled) - current)
    console.print("Packs detectados: " + ", ".join(match.pack.name for match in matches))
    if not result.changed:
        console.print("[green]✓[/] Todas las skills compatibles ya están habilitadas.")
        return
    status = "se añadirían" if dry_run else "añadidas"
    console.print(f"[green]✓[/] Skills únicas {status}: {', '.join(added)}")
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
    if not dry_run:
        console.print("Carga las instrucciones con: chxchx-tech skill sync .")
