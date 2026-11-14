from __future__ import annotations

import json
from pathlib import Path

import typer
from rich.table import Table

from ..cli_registry import mod_app
from ..core.mods import ModRegistry, apply_mods, project_mod_status, remove_mods
from ..ui.cli_output import console


@mod_app.command("list")
def mod_list(
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    json_output: bool = typer.Option(False, "--json", help="Devuelve catálogo y estado como JSON para Studio."),
) -> None:
    """Lista los Mods incluidos y marca los recomendados para el proyecto."""
    if json_output:
        console.print(json.dumps({
            "schema": "chxchx.mods",
            "schema_version": 1,
            "mods": project_mod_status(path),
        }, ensure_ascii=False), markup=False, soft_wrap=True)  # a console would wrap the line inside the JSON strings
        return
    recommended = {item.mod.name: item for item in ModRegistry().recommendations(path)}
    table = Table(title="Mods · locales al proyecto (Claude Code, Codex y OpenCode)")
    table.add_column("Mod", style="cyan")
    table.add_column("Descripción")
    table.add_column("Recomendación")
    for mod in ModRegistry().list():
        match = recommended.get(mod.name)
        table.add_row(mod.name, mod.description, "; ".join(match.reasons) if match else "—")
    console.print(table)
    console.print("Son instrucciones Markdown; no instalan plugins ejecutables ni cambian configuración global de ningún agente.")


@mod_app.command("apply")
def mod_apply(
    names: list[str] = typer.Argument(None, help="IDs a instalar; si se omiten, aplica los recomendados."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza sin escribir archivos ni backups."),
) -> None:
    """Instala Mods como skills del proyecto: .claude/skills (Claude Code y OpenCode) y .agents/skills (Codex)."""
    try:
        result = apply_mods(path, tuple(names) if names else None, dry_run=dry_run)
    except (OSError, ValueError) as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    status = "se instalarían" if dry_run else "instalados"
    console.print(f"[green]✓[/] Mods {status}: {', '.join(result.installed) or 'sin cambios'}")
    if result.unchanged:
        console.print("Sin cambios: " + ", ".join(result.unchanged))
    for backup in result.backups:
        console.print(f"Backup: {backup}")
    if result.installed:
        console.print("Los agentes los reconocen al iniciar una sesión nueva; en Claude Code invócalos con /chxchx-<nombre>.")
    if dry_run:
        console.print("No se realizaron cambios.")


@mod_app.command("remove")
def mod_remove(
    names: list[str] = typer.Argument(..., help="IDs de Mods administrados por ChxChx a retirar."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza la retirada sin escribir backups ni borrar."),
) -> None:
    """Retira Mods de ChxChx guardando primero un backup recuperable."""
    try:
        result = remove_mods(path, tuple(names), dry_run=dry_run)
    except (OSError, ValueError) as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    status = "se retirarían" if dry_run else "retirados"
    console.print(f"[green]✓[/] Mods {status}: {', '.join(result.installed) or 'ninguno'}")
    for backup in result.backups:
        console.print(f"Backup recuperable: {backup}")
    if dry_run:
        console.print("No se realizaron cambios.")
