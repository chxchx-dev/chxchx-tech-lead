from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import typer
from rich.table import Table

from ..cli_registry import local_models_app
from ..integrations.local_model_fit import DEFAULT_CONTEXT
from ..integrations.local_models_remove import remove_local_model
from ..integrations.local_models_setup import LocalModelSetupError, setup_local_model
from ..ui.cli_output import console
from ..workspace.bridge_contracts import SCHEMA_VERSION
from ..workspace.bridge_local_models import current_hardware, local_models_payload

_FIT_LABELS = {"gpu": "[green]GPU[/]", "partial": "[yellow]GPU+RAM[/]", "cpu": "[yellow]CPU[/]", "none": "[red]No cabe[/]"}
EVENT_SCHEMA = "chxchx.local-models-event"


def _emitter(as_json: bool):
    def emit(event: dict[str, Any]) -> None:
        if as_json:
            typer.echo(json.dumps({"schema": EVENT_SCHEMA, "schema_version": SCHEMA_VERSION, **event}, ensure_ascii=False))
        elif event["type"] == "preview":
            console.print(f"[bold]{event['path']}[/]\n{event['content']}")
        elif event["type"] == "notice":
            console.print(event["message"])
        elif event["type"] == "step":
            console.print(f"→ {event['step']}: {event['model']}")
        elif event["type"] == "progress" and event["total"] and event["completed"] == event["total"]:
            console.print(f"  {event['status']} 100%")

    return emit


@local_models_app.command("status")
def local_models_status(
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Muestra tu hardware, el estado de Ollama y qué modelos locales caben."""
    payload = local_models_payload(path)
    hardware, ollama = payload["hardware"], payload["ollama"]
    gib = 1024**3
    console.print(f"Hardware: {hardware['total_ram_bytes'] / gib:.0f} GB RAM ({hardware['available_ram_bytes'] / gib:.0f} GB libres), "
                  f"{hardware['cpu_cores']} núcleos, {hardware['accelerator']}, {hardware['disk_free_bytes'] / gib:.0f} GB de disco libres.")
    if not ollama["installed"]:
        install = ollama["install"]
        console.print("[yellow]Ollama no está instalado.[/] Instálalo con el método oficial: "
                      f"{install['command'] or install['url']}")
    elif not ollama["running"]:
        console.print(f"[yellow]Ollama está instalado pero no responde en {ollama['base_url']}.[/] Inicia la aplicación o `ollama serve`.")
    else:
        console.print(f"Ollama {ollama['version']} activo en {ollama['base_url']} ({len(ollama['models'])} modelos).")
    if not payload["opencode"]["installed"]:
        console.print(f"[yellow]OpenCode no está instalado:[/] {payload['opencode']['install_hint']}")
    table = Table(title="Modelos locales (estimación)")
    for column in ("Modelo", "Descarga", "Encaje", "Estado", "Detalle"):
        table.add_column(column)
    for model in payload["models"]:
        state = "configurado" if model["configured"] else "descargado" if model["downloaded"] else ""
        mark = " ★" if model["recommended"] else ""
        table.add_row(model["tag"] + mark, f"{model['size_bytes'] / gib:.1f} GB", _FIT_LABELS[model["fit"]], state, model["summary"])
    console.print(table)
    console.print("★ recomendado para este equipo. Configura uno con: chxchx-tech local-models setup MODELO --dry-run")


@local_models_app.command("setup")
def local_models_setup(
    model: str = typer.Argument(..., help="Etiqueta del catálogo, por ejemplo qwen3-coder:30b."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    context: int = typer.Option(DEFAULT_CONTEXT, "--context", min=8192, help="Ventana de contexto en tokens."),
    set_default: bool = typer.Option(False, "--set-default", help="Usa este modelo como predeterminado de OpenCode en el proyecto."),
    yes: bool = typer.Option(False, "--yes", help="Confirma la descarga del modelo si falta."),
    force: bool = typer.Option(False, "--force", help="Continúa aunque el modelo no quepa en este equipo."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra los pasos y el opencode.json resultante sin cambiar nada."),
    as_json: bool = typer.Option(False, "--json", help="Emite eventos JSONL (uso de Studio)."),
) -> None:
    """Descarga el modelo en Ollama, amplía su contexto y lo conecta a OpenCode en este proyecto."""
    emit = _emitter(as_json)
    confirmed = yes or (not dry_run and not as_json and sys.stdin.isatty()
                        and typer.confirm(f"Si {model} no está descargado se bajará con `ollama pull`. ¿Continuar?"))
    try:
        result = setup_local_model(path, model, current_hardware(), emit, context=context, set_default=set_default,
                                   confirmed=confirmed, dry_run=dry_run, force=force)
    except LocalModelSetupError as exc:
        if as_json:
            emit({"type": "error", "message": str(exc)})
        else:
            console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    if as_json:
        emit({"type": "done", "model": result.model_ref, "changed": result.changed, "config_path": str(result.config_path),
              "backup_path": str(result.backup_path) if result.backup_path else None, "dry_run": dry_run})
        return
    verb = "DRY RUN: se configuraría" if dry_run else "Configurado" if result.changed else "Ya estaba configurado"
    console.print(f"[green]✓[/] {verb} `{result.model_ref}` en {result.config_path}")
    if result.backup_path:
        console.print(f"Backup del opencode.json anterior: {result.backup_path}")
    if not dry_run:
        console.print(f"Úsalo en OpenCode con el modelo `{result.model_ref}`" + ("" if set_default else " (o repite con --set-default)."))


@local_models_app.command("remove")
def local_models_remove(
    model: str = typer.Argument(..., help="Etiqueta del catálogo, por ejemplo qwen3-coder:30b."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    yes: bool = typer.Option(False, "--yes", help="Confirma el borrado del modelo."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra qué se borraría sin cambiar nada."),
    as_json: bool = typer.Option(False, "--json", help="Emite eventos JSONL (uso de Studio)."),
) -> None:
    """Borra el modelo de Ollama (y sus variantes de contexto) y lo quita del opencode.json del proyecto."""
    emit = _emitter(as_json)
    confirmed = yes or (not dry_run and not as_json and sys.stdin.isatty()
                        and typer.confirm(f"Se borrará {model} de Ollama y de opencode.json. ¿Continuar?"))
    try:
        result = remove_local_model(path, model, emit, confirmed=confirmed, dry_run=dry_run)
    except LocalModelSetupError as exc:
        if as_json:
            emit({"type": "error", "message": str(exc)})
        else:
            console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1) from exc
    if as_json:
        emit({"type": "done", "model": model, "removed": list(result.removed), "changed": result.config_changed,
              "config_path": str(result.config_path), "backup_path": str(result.backup_path) if result.backup_path else None,
              "dry_run": dry_run})
        return
    verb = "DRY RUN: se quitaría" if dry_run else "Quitado"
    console.print(f"[green]✓[/] {verb} {model} ({', '.join(result.removed) or 'sin archivos en Ollama'}).")
    if result.backup_path:
        console.print(f"Backup del opencode.json anterior: {result.backup_path}")
