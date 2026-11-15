from __future__ import annotations

import json
import sys
from pathlib import Path

import typer

from ..cli_registry import agent_app
from ..workspace.chat_state import reset_state, state_payload


@agent_app.command("chat-state")
def agent_chat_state(
    agent_id: str = typer.Argument(..., metavar="agent_id"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    reset: bool = typer.Option(False, "--reset", help="Olvida la conversación guardada (chat nuevo)."),
    restore: bool = typer.Option(False, "--restore",
        help="Lee la conversación para repintar el chat y marca su sesión para comprobarla en el siguiente turno (uso de Studio)."),
) -> None:
    """Conversación guardada del chat visual de un agente (JSON `chxchx.chat-state` v1)."""
    if reset:
        reset_state(path, agent_id)
    # Plain stdout: a console wraps long lines, and a conversation is long.
    sys.stdout.write(json.dumps(state_payload(path, agent_id, restoring=restore and not reset), ensure_ascii=False) + "\n")
