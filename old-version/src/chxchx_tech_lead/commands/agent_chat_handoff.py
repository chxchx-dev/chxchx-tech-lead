"""Internal read-only provider turn used by Rust Studio to prepare a context handoff."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import typer

from ..cli_registry import agent_app
from ..workspace.agent_chat import json_line_emitter, load_prompt, plan_chat_turn, run_chat_turn
from ..workspace.project_context import workspace_service_for
from ..workspace.service import WorkspaceOperationError


@agent_app.command("chat-handoff", hidden=True)
def agent_chat_handoff(
    agent_id: str = typer.Argument(...),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    session: str = typer.Option(..., "--session"),
    prompt_file: Path = typer.Option(..., "--prompt-file", exists=True, dir_okay=False, resolve_path=True),
    cancel_on_stdin: bool = typer.Option(False, "--cancel-on-stdin"),
    model: str = typer.Option("", "--model"),
    effort: str = typer.Option("", "--effort"),
) -> None:
    """Generate one handoff without changing the visible transcript or project files."""
    emit = json_line_emitter()
    try:
        prompt = load_prompt("", prompt_file)
        plan = plan_chat_turn(
            workspace_service_for(path),
            agent_id,
            mode="read-only",
            session_id=session,
            model=model or None,
            effort=effort or None,
            network=False,
            allow=(),
            live=False,
            auto_release=False,
            resume_decided_by_rust=True,
            rust_chat_handoff=True,
        )
    except WorkspaceOperationError as exc:
        emit({"type": "error", "message": str(exc)})
        raise typer.Exit(code=1)

    messages: list[str] = []
    cancelled = False

    def capture(event: dict[str, Any]) -> None:
        nonlocal cancelled
        if event.get("type") == "message" and isinstance(event.get("text"), str):
            messages.append(event["text"])
        elif event.get("type") == "housekeeping_done":
            cancelled = bool(event.get("cancelled"))

    code = run_chat_turn(
        plan,
        prompt,
        capture,
        cancel_input=sys.stdin if cancel_on_stdin else None,
        housekeeping=True,
    )
    summary = "\n\n".join(messages).strip()[:24_000]
    if code != 0 or cancelled or not summary:
        emit({"type": "error", "message": "No se pudo generar el traspaso de contexto."})
        raise typer.Exit(code=1)
    emit({"type": "handoff", "text": summary})
