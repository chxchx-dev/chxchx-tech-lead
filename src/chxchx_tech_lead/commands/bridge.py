from __future__ import annotations

import json
from pathlib import Path

import typer

from ..cli_registry import bridge_app
from ..workspace.bridge import project_status_payload
from ..workspace.bridge_contracts import ERROR_SCHEMA, SCHEMA_VERSION
from ..workspace.bridge_resources import resources_overview_payload
from ..workspace.bridge_history import (
    conversation_payload,
    conversations_payload,
    errors_payload,
    handoff_payload,
    memory_payload,
)
from ..workspace.service import WorkspaceOperationError


@bridge_app.command("status")
def bridge_status(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Imprime el estado del proyecto con el esquema JSON versionado."""
    try:
        payload = project_status_payload(path)
    except (OSError, ValueError, WorkspaceOperationError) as exc:
        typer.echo(json.dumps({
            "schema": ERROR_SCHEMA,
            "schema_version": SCHEMA_VERSION,
            "error": str(exc),
        }, ensure_ascii=False, separators=(",", ":")))
        raise typer.Exit(code=1) from exc
    typer.echo(json.dumps(payload, ensure_ascii=False, separators=(",", ":")))


@bridge_app.command("resources")
def bridge_resources(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Imprime el snapshot del sistema y consumo agregado por proyecto."""
    _emit_payload(resources_overview_payload, path)


@bridge_app.command("handoff")
def bridge_handoff(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Lee el handoff del proyecto en JSON, sin modificarlo."""
    _emit_payload(handoff_payload, path)


@bridge_app.command("memory")
def bridge_memory(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    query: str = typer.Option("", "--query", help="Filtra por título o contenido."),
) -> None:
    """Busca y lee notas locales del proyecto, sin modificarlas."""
    _emit_payload(memory_payload, path, query)


@bridge_app.command("chats")
def bridge_chats(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    query: str = typer.Option("", "--query", help="Filtra por título o contenido."),
) -> None:
    """Lista conversaciones locales asociadas a este proyecto."""
    _emit_payload(conversations_payload, path, query)


@bridge_app.command("conversation")
def bridge_conversation(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    provider: str = typer.Argument(..., help="Codex o Claude."),
    session_id: str = typer.Argument(..., help="ID de la conversación."),
) -> None:
    """Lee los mensajes de una conversación local concreta."""
    _emit_payload(conversation_payload, path, provider, session_id)


@bridge_app.command("errors")
def bridge_errors(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Lista errores locales asociados a este proyecto."""
    _emit_payload(errors_payload, path)


def _emit_payload(builder, *args) -> None:
    try:
        payload = builder(*args)
    except (OSError, UnicodeError, ValueError, WorkspaceOperationError) as exc:
        typer.echo(json.dumps({
            "schema": ERROR_SCHEMA,
            "schema_version": SCHEMA_VERSION,
            "error": str(exc),
        }, ensure_ascii=False, separators=(",", ":")))
        raise typer.Exit(code=1) from exc
    typer.echo(json.dumps(payload, ensure_ascii=False, separators=(",", ":")))
