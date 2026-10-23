from __future__ import annotations

import json
from pathlib import Path

import typer
from rich.table import Table

from ..cli_registry import ci_app
from ..integrations.ci_service import ci_logs, ci_status_payload, remove_ci, run_ci, setup_ci
from ..integrations.github_cli import GitHubCliError
from ..ui.cli_output import console


@ci_app.command("setup")
def setup_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el workflow detectado sin escribirlo."),
) -> None:
    """Prepara un workflow CI opt-in para Python, Node o CMake/C++/Qt."""
    try:
        result, plan = setup_ci(path, dry_run=dry_run)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    console.print(f"Proyecto: {path.resolve()}")
    console.print(f"Stack cubierto: {', '.join(plan.stacks)}")
    console.print(f"Workflow: {result.path}")
    for warning in plan.warnings:
        console.print(f"[yellow]Aviso:[/] {warning}")
    if not result.changed:
        console.print("[green]El workflow administrado ya está actualizado.[/]")
        return
    if dry_run:
        console.print("[yellow]Vista previa; no se escribió ningún archivo.[/]")
        console.print(result.content, markup=False)
        return
    if result.backup:
        console.print(f"Backup anterior: {result.backup}")
    console.print("Workflow CI guardado. Para activarlo en GitHub, confirma y sube este archivo al repositorio.")


@ci_app.command("status")
def status_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    as_json: bool = typer.Option(False, "--json", help="Imprime el contrato JSON de estado."),
) -> None:
    """Consulta configuración local y las últimas ejecuciones de GitHub Actions."""
    try:
        payload = ci_status_payload(path)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    if as_json:
        console.print(json.dumps(payload, ensure_ascii=False, separators=(",", ":")), markup=False)
        return
    table = Table(title="ChxChx · CI/CD")
    table.add_column("Elemento")
    table.add_column("Estado")
    table.add_row("Workflow", "activo local" if payload["configured"] else "desactivado")
    table.add_row("Integridad", "válida" if payload["integrity_ok"] else "no disponible")
    table.add_row("GitHub CLI", "conectado" if payload["authenticated"] else "sin sesión")
    for stack in payload["supported_stacks"]:
        table.add_row("Stack", str(stack))
    console.print(table)
    if payload["message"]:
        console.print(str(payload["message"]))
    for warning in payload["warnings"]:
        console.print(f"[yellow]Aviso:[/] {warning}")
    runs = payload["recent_runs"]
    if runs:
        recent = Table(title="Ejecuciones recientes")
        for column in ("ID", "Workflow", "Rama", "Estado", "Resultado", "URL"):
            recent.add_column(column)
        for item in runs:
            recent.add_row(*(
                str(item.get(key, ""))
                for key in ("databaseId", "workflowName", "headBranch", "status", "conclusion", "url")
            ))
        console.print(recent)


@ci_app.command("remove")
def remove_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza la retirada sin borrar el archivo."),
) -> None:
    """Retira solo el workflow íntegro que administra ChxChx, con backup."""
    try:
        result = remove_ci(path, dry_run=dry_run)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    if not result.changed:
        console.print("No hay un workflow administrado para retirar.")
        return
    console.print(f"Workflow: {result.path}")
    if dry_run:
        console.print("[yellow]Vista previa; el archivo sigue intacto.[/]")
        console.print(result.content, markup=False)
        return
    console.print(f"Workflow retirado. Backup: {result.backup}")


@ci_app.command("run")
def run_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    ref: str | None = typer.Option(None, "--ref", help="Rama o tag que contiene el workflow."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza sin contactar GitHub."),
) -> None:
    """Inicia manualmente el workflow CI de este proyecto mediante gh."""
    try:
        result = run_ci(path, ref, dry_run=dry_run)
    except (GitHubCliError, OSError, ValueError) as exc:
        _fail(str(exc))
    if result.stdout:
        console.print(result.stdout, markup=False)
    else:
        console.print("CI solicitado; consulta `chxchx-tech ci status` para seguirlo.")


@ci_app.command("logs")
def logs_command(
    run_id: str = typer.Argument(..., help="ID numérico de una ejecución GitHub Actions."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Muestra hasta 96 KB de los logs fallidos de una ejecución."""
    try:
        result = ci_logs(path, run_id)
    except (GitHubCliError, OSError, ValueError) as exc:
        _fail(str(exc))
    console.print(result.stdout or "La ejecución no contiene logs fallidos.", markup=False)


def _fail(message: str) -> None:
    console.print(f"[red]Error:[/] {message}")
    raise typer.Exit(code=1)
