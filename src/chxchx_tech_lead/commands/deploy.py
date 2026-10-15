from __future__ import annotations

import json
from pathlib import Path

import typer
from rich.table import Table

from ..cli_registry import deploy_app
from ..integrations.ci_service import ci_logs
from ..integrations.deploy_service import (
    deployment_status_payload,
    remove_deployment,
    run_deployment,
    setup_deployment,
)
from ..integrations.github_cli import GitHubCliError
from ..ui.cli_output import console


@deploy_app.command("setup")
def setup_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    auto_deploy_branch: str | None = typer.Option(
        None,
        "--auto-deploy-on",
        help="Habilita CD automático al subir cambios a esta rama. Sin la opción, solo será manual.",
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza el workflow sin escribirlo."),
) -> None:
    """Prepara CD opcional por SSH para Docker Compose en un VPS."""
    try:
        result, plan = setup_deployment(path, auto_deploy_branch, dry_run=dry_run)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    console.print(f"Proyecto: {path.resolve()}")
    console.print(f"Runtime: Docker Compose ({plan.compose_file})")
    console.print("Modo: manual" if not plan.auto_deploy_branch else f"Automático al actualizar: {plan.auto_deploy_branch}")
    console.print(f"Workflow: {result.path}")
    console.print("Variables de GitHub Actions: " + ", ".join(plan.required_variables))
    console.print("Secretos de GitHub Actions: " + ", ".join(plan.required_secrets))
    if not result.changed:
        console.print("[green]El workflow de despliegue ya está actualizado.[/]")
        return
    if dry_run:
        console.print("[yellow]Vista previa; no se escribió ningún archivo.[/]")
        console.print(result.content, markup=False)
        return
    if result.backup:
        console.print(f"Backup anterior: {result.backup}")
    console.print(
        "Workflow guardado. Antes de ejecutarlo, configura las variables y secretos indicados, "
        "prepara el clon del repositorio en el VPS y verifica su llave SSH."
    )


@deploy_app.command("status")
def status_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    as_json: bool = typer.Option(False, "--json", help="Imprime el contrato JSON de estado."),
) -> None:
    """Consulta la configuración local y las ejecuciones del workflow de CD."""
    try:
        payload = deployment_status_payload(path)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    if as_json:
        console.print(json.dumps(payload, ensure_ascii=False, separators=(",", ":")), markup=False)
        return
    table = Table(title="ChxChx · CD a VPS")
    table.add_column("Elemento")
    table.add_column("Estado")
    table.add_row("Workflow", "activo local" if payload["configured"] else "desactivado")
    table.add_row("Integridad", "válida" if payload["integrity_ok"] else "no disponible")
    table.add_row("Runtime", str(payload["runtime"] or "no detectado"))
    table.add_row("GitHub CLI", "conectado" if payload["authenticated"] else "sin sesión")
    if payload["auto_deploy_branch"]:
        table.add_row("Despliegue automático", str(payload["auto_deploy_branch"]))
    console.print(table)
    if payload["message"]:
        console.print(str(payload["message"]))
    for warning in payload["warnings"]:
        console.print(f"[yellow]Aviso:[/] {warning}")
    runs = payload["recent_runs"]
    if runs:
        recent = Table(title="Despliegues recientes")
        for column in ("ID", "Rama", "Estado", "Resultado", "URL"):
            recent.add_column(column)
        for item in runs:
            recent.add_row(*(str(item.get(key, "")) for key in (
                "databaseId", "headBranch", "status", "conclusion", "url"
            )))
        console.print(recent)


@deploy_app.command("run")
def run_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    ref: str | None = typer.Option(None, "--ref", help="Rama o tag que contiene el workflow y el código."),
    confirm: bool = typer.Option(False, "--confirm", help="Confirma el despliegue remoto al VPS."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza sin contactar GitHub."),
) -> None:
    """Despacha un despliegue manual; exige --confirm fuera de dry-run."""
    try:
        result = run_deployment(path, ref, confirmed=confirm, dry_run=dry_run)
    except (GitHubCliError, OSError, ValueError) as exc:
        _fail(str(exc))
    if dry_run:
        console.print(result.stdout, markup=False)
        if not confirm:
            console.print("Para enviar el despliegue tendrás que revisar y añadir --confirm.")
    elif result.stdout:
        console.print(result.stdout, markup=False)
    else:
        console.print("Despliegue solicitado; consulta `chxchx-tech deploy status` para seguirlo.")


@deploy_app.command("remove")
def remove_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza la retirada sin borrar el workflow."),
) -> None:
    """Retira solo el workflow íntegro que administra ChxChx, con backup."""
    try:
        result = remove_deployment(path, dry_run=dry_run)
    except (OSError, ValueError) as exc:
        _fail(str(exc))
    if not result.changed:
        console.print("No hay un workflow de despliegue administrado para retirar.")
        return
    console.print(f"Workflow: {result.path}")
    if dry_run:
        console.print("[yellow]Vista previa; el archivo sigue intacto.[/]")
        console.print(result.content, markup=False)
        return
    console.print(f"Workflow retirado. Backup: {result.backup}")


@deploy_app.command("logs")
def logs_command(
    run_id: str = typer.Argument(..., help="ID numérico de una ejecución GitHub Actions."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Muestra hasta 96 KB de los logs fallidos de un despliegue."""
    try:
        result = ci_logs(path, run_id)
    except (GitHubCliError, OSError, ValueError) as exc:
        _fail(str(exc))
    console.print(result.stdout or "La ejecución no contiene logs fallidos.", markup=False)


def _fail(message: str) -> None:
    console.print(f"[red]Error:[/] {message}")
    raise typer.Exit(code=1)
