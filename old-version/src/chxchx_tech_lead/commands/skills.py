from __future__ import annotations

from pathlib import Path

import typer
from rich.table import Table

from ..cli_registry import skill_app
from ..skills import Skill, SkillRegistry, enabled_skills, set_skill_enabled, sync_project_skills
from ..ui.cli_output import console


def _show_skills(skills: list[Skill], title: str) -> None:
    if not skills:
        console.print("[yellow]No se encontraron skills.[/]")
        return
    table = Table(title=title)
    table.add_column("Skill", style="cyan")
    table.add_column("Descripción")
    table.add_column("Stack")
    for skill in skills:
        table.add_row(skill.name, skill.description, ", ".join(skill.stacks or skill.languages) or "general")
    console.print(table)

def _show_recommendations(recommendations, title: str) -> None:
    if not recommendations:
        console.print("[yellow]No se encontraron skills compatibles.[/]")
        return
    table = Table(title=title)
    table.add_column("Skill", style="cyan")
    table.add_column("Descripción")
    table.add_column("Motivo")
    for item in recommendations:
        table.add_row(item.skill.name, item.skill.description, "; ".join(item.reasons))
    console.print(table)


@skill_app.command("list")
def skill_list() -> None:
    """Lista las skills disponibles en la biblioteca local."""
    skills = SkillRegistry().list()
    _show_skills(skills, "Skill Registry")
    console.print(f"{len(skills)} skills disponibles; el catálogo viene incluido con ChxChx.")


@skill_app.command("search")
def skill_search(query: str = typer.Argument(..., help="Texto a buscar en nombres, descripciones y tags.")) -> None:
    """Busca skills por nombre, descripción o tag."""
    _show_skills(SkillRegistry().search(query), f"Skills: {query}")


@skill_app.command("info")
def skill_info(name: str = typer.Argument(..., help="Nombre de la skill.")) -> None:
    """Muestra metadatos e instrucciones de una skill."""
    skill = SkillRegistry().get(name)
    if skill is None:
        console.print(f"[red]Skill desconocida:[/] {name}")
        raise typer.Exit(code=1)
    console.print(f"[bold cyan]{skill.name}[/] — {skill.description}")
    console.print(f"Origen: {skill.origin} · Stack: {', '.join(skill.stacks or skill.languages) or 'general'}")
    console.print(skill.instructions)


@skill_app.command("recommend")
def skill_recommend(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Detecta el stack y explica por qué recomienda cada skill."""
    recommendations = SkillRegistry().recommendations(path)
    if recommendations:
        console.print(f"Stack detectado en [bold]{path.name}[/]")
    _show_recommendations(recommendations, "Skills recomendadas")

@skill_app.command("status")
def skill_status(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Muestra cuántas skills están habilitadas para el proyecto."""
    try:
        names = enabled_skills(path)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    registry = SkillRegistry()
    total = len(registry.list())
    console.print(f"{len(names)} de {total} skills habilitadas en {path.name}")
    for name in names:
        skill = registry.get(name)
        if skill is None:
            console.print(f"[yellow]! Skill fuera del catálogo: {name}[/]")
        else:
            console.print(f"[green]✓[/] {skill.name}: {skill.description}")


@skill_app.command("enable")
def skill_enable(
    name: str = typer.Argument(..., help="Nombre de la skill."),
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el cambio sin escribirlo."),
) -> None:
    """Habilita una skill para el proyecto, sin sincronizarla todavía."""
    try:
        result = set_skill_enabled(path, name, True, dry_run=dry_run)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    status = "se habilitaría" if dry_run else "habilitada"
    console.print(f"[green]✓[/] {name} {status} en {path}")
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
    if result.changed and not dry_run:
        console.print("Sincroniza las instrucciones con: chxchx-tech skill sync .")
    elif result.changed:
        console.print("Repite sin --dry-run para guardar la selección y luego sincronízala.")


@skill_app.command("disable")
def skill_disable(
    name: str = typer.Argument(..., help="Nombre de la skill."),
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el cambio sin escribirlo."),
) -> None:
    """Deshabilita una skill para el proyecto."""
    try:
        result = set_skill_enabled(path, name, False, dry_run=dry_run)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    status = "se deshabilitaría" if dry_run else "deshabilitada"
    console.print(f"[green]✓[/] {name} {status} en {path}")
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
    if result.changed and not dry_run:
        console.print("Sincroniza el contexto actualizado con: chxchx-tech skill sync .")
    elif result.changed:
        console.print("Repite sin --dry-run para guardar la selección y luego sincronízala.")


@skill_app.command("sync")
def skill_sync(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra los archivos afectados sin escribirlos."),
) -> None:
    """Escribe las instrucciones habilitadas en .ai/SKILLS.md."""
    try:
        result = sync_project_skills(path, dry_run=dry_run)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    status = "sin cambios" if not result.changed else ("sincronización prevista" if dry_run else "contexto sincronizado")
    console.print(f"[green]✓[/] {status}: {', '.join(result.enabled) or 'sin skills habilitadas'}")
    if result.changed:
        console.print(f"Archivos: {path / '.ai' / 'SKILLS.md'} y {path / 'AGENTS.md'}")
    if result.backup:
        console.print(f"[dim]Backup: {result.backup}[/]")
