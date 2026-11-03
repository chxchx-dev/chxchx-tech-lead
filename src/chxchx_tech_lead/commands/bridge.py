from __future__ import annotations

import json
from pathlib import Path

import typer

from ..cli_registry import bridge_app
from ..workspace.agent_chat import agent_options_payload
from ..workspace.bridge import project_status_payload
from ..workspace.project_context import workspace_service_for
from ..workspace.bridge_contracts import ERROR_SCHEMA, SCHEMA_VERSION
from ..workspace.bridge_local_models import local_models_payload
from ..workspace.bridge_resources import resources_overview_payload
from ..integrations.ci_service import ci_status_payload
from ..integrations.deploy_service import deployment_status_payload
from ..integrations.studio_update_install import apply_update, update_status
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


@bridge_app.command("ci")
def bridge_ci(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Lee el estado de CI/CD local y ejecuciones remotas sin modificarlas."""
    _emit_payload(ci_status_payload, path)


@bridge_app.command("deploy")
def bridge_deploy(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Lee el estado local de CD a VPS y las ejecuciones remotas, sin modificarlas."""
    _emit_payload(deployment_status_payload, path)


@bridge_app.command("studio-update")
def bridge_studio_update(
    apply: bool = typer.Option(False, "--apply", help="Instala la versión publicada tras confirmación del usuario."),
) -> None:
    """Consulta actualizaciones de Studio o instala explícitamente la publicada."""
    _emit_payload(apply_update if apply else update_status)


@bridge_app.command("agent-options")
def bridge_agent_options(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    agent: str = typer.Option(..., "--agent", help="ID del agente Codex o Claude."),
) -> None:
    """Lista modelos, niveles de effort y disponibilidad de un agente para el chat visual."""
    _emit_payload(lambda: agent_options_payload(workspace_service_for(path), agent))


@bridge_app.command("local-models")
def bridge_local_models(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
) -> None:
    """Hardware, estado de Ollama y modelos locales recomendados para OpenCode."""
    _emit_payload(local_models_payload, path)


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
