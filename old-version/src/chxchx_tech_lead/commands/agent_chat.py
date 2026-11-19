"""The structured agent chat command used by Studio and the terminal."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import typer

from ..cli_registry import agent_app
from ..integrations.agent_stream import MODES
from ..workspace.agent_chat import json_line_emitter, load_prompt, plan_chat_turn, run_chat_turn
from ..workspace.chat_context_bridge import load_rust_context
from ..workspace.chat_resume import load_resume_recap
from ..workspace.project_context import workspace_service_for as _workspace_service
from ..workspace.service import WorkspaceOperationError
from .agent_errors import _record_agent_error


@agent_app.command("chat")
def agent_chat(
    agent_id: str = typer.Argument(..., help="ID del agente Codex, Claude u OpenCode configurado."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    prompt: str = typer.Option("", "--prompt", help="Mensaje del usuario para este turno."),
    prompt_file: Path | None = typer.Option(None, "--prompt-file", exists=True, dir_okay=False, resolve_path=True,
        help="Lee el mensaje de un archivo (para mensajes largos, como un diff)."),
    lean: bool = typer.Option(False, "--lean",
        help="Consulta puntual: sin contexto del proyecto, memoria ni diario, sin guardar la sesión y siempre en solo lectura."),
    session: str = typer.Option("", "--session", help="Sesión a continuar; vacío inicia un chat con contexto."),
    resume_decided_by_rust: bool = typer.Option(False, "--resume-decided-by-rust",
        help="Studio ya verificó la sesión y decidió si reconstruir el contexto."),
    resume_recap_file: Path | None = typer.Option(None, "--resume-recap-file", exists=True, dir_okay=False, resolve_path=True,
        help="Lee el recap de una sesión perdida preparado por Studio."),
    rust_context_file: Path | None = typer.Option(None, "--rust-context-file", exists=True, dir_okay=False, resolve_path=True,
        help="Lee el contexto inicial acotado preparado por Studio."),
    rust_chat_handoff: bool = typer.Option(False, "--rust-chat-handoff",
        help="Studio Rust gestiona la liberación y el consumo de traspasos."),
    mode: str = typer.Option("read-only", "--mode", help=f"Permisos del agente: {', '.join(MODES)}."),
    model: str = typer.Option("", "--model", help="Modelo; vacío usa el configurado en el agente."),
    effort: str = typer.Option("", "--effort", help="Nivel de razonamiento; vacío usa el configurado."),
    images: list[Path] = typer.Option([], "--image", help="Imagen adjunta (repetible)."),
    network: bool = typer.Option(False, "--network", help="Permite al agente usar la red / la web en este turno."),
    allow: list[str] = typer.Option([], "--allow", help="Permiso ya aprobado, tal como lo propone un evento `permission` (repetible)."),
    live: bool = typer.Option(False, "--live",
        help="Claude pregunta durante el turno (permisos y preguntas); las respuestas llegan por stdin (uso de Studio)."),
    cancel_on_stdin: bool = typer.Option(False, "--cancel-on-stdin",
        help="Detiene el agente al leer `cancel` o EOF por stdin (uso de Studio)."),
    auto_release: bool = typer.Option(True, "--auto-release/--no-auto-release",
        help="Libera el contexto solo al pasar del 90 %: guarda un traspaso y la siguiente sesión continúa con él."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el comando del turno sin ejecutarlo."),
):
    """Ejecuta un turno de chat estructurado y emite eventos JSONL `chxchx.agent-event` v1."""
    emit = json_line_emitter()
    try:
        text = load_prompt(prompt, prompt_file)
        resume_recap = load_resume_recap(resume_recap_file)
        rust_context, compacted_sections = load_rust_context(rust_context_file)
        plan = plan_chat_turn(_workspace_service(path), agent_id, mode=mode, session_id=session or None,
                              model=model or None, effort=effort or None, images=tuple(images), lean=lean,
                              network=network, allow=tuple(allow), live=live and cancel_on_stdin, auto_release=auto_release,
                              resume_decided_by_rust=resume_decided_by_rust, resume_recap=resume_recap,
                              rust_chat_handoff=rust_chat_handoff)
    except WorkspaceOperationError as exc:
        _record_agent_error(path, "agent chat", str(exc))
        emit({"type": "error", "message": str(exc)})
        emit({"type": "done", "exit_code": 1, "cancelled": False, "session_id": session or None})
        raise typer.Exit(code=1)
    if dry_run:
        emit({"type": "plan", "provider": plan.provider, "argv": plan.argv, "cwd": str(plan.cwd), "mode": plan.mode})
        return
    cancelled = False

    def emit_chat_event(event: dict[str, Any]) -> None:
        nonlocal cancelled
        emit(event)
        if event.get("type") == "notice" and event.get("error") is True:
            _record_agent_error(path, f"agent chat · {plan.provider}", str(event.get("message", "")))
        if event.get("type") == "done":
            cancelled = bool(event.get("cancelled"))

    try:
        exit_code = run_chat_turn(
            plan, text, emit_chat_event,
            cancel_input=sys.stdin if cancel_on_stdin else None,
            rust_context=rust_context,
            rust_context_compacted_sections=compacted_sections,
        )
    except Exception as exc:
        _record_agent_error(path, f"agent chat · {plan.provider}", f"Fallo interno {type(exc).__name__}: {exc}")
        raise
    if exit_code != 0:
        if not cancelled:
            _record_agent_error(path, f"agent chat · {plan.provider}", f"{plan.provider} terminó con código {exit_code}.")
        raise typer.Exit(code=1)
