"""Run one visual-chat turn of a configured agent and stream normalized events.

Studio's chat panel starts `chxchx-tech agent chat` per user message. This use
case keeps the authority in Python: trust, context, memory capture and the
session journal are the same ones the terminal agent pane uses.
"""

from __future__ import annotations

import json
import os
import secrets
import subprocess
import sys
import threading
from collections import deque
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import IO, Any

from ..integrations.agent_access import Access, opencode_agent
from ..integrations.agent_codex_live import LiveCodex, configured_model
from ..integrations.agent_live import LiveClaude
from ..integrations.agent_opencode_live import LiveOpenCode
from ..integrations.agent_catalog import INSTALL_HINTS, agent_options, codex_limits, executable_available, resolve_executable
from ..integrations.agent_stream import IMAGE_ARGUMENT_PROVIDERS, normalize, provider_for, turn_command, turn_environment
from .agent_commands import CHAT_PANEL_REQUEST_MARKER, new_chat_prompt
from .bridge_contracts import AGENT_OPTIONS_SCHEMA, SCHEMA_VERSION as BRIDGE_SCHEMA_VERSION
from .chat_attachments import stage_images, validated_image
from .chat_process import delete_opencode_session, drain, watch_cancel
from .chat_context import capture_after_session, inline_project_context, prepare_new_chat
from .service import WorkspaceOperationError

SCHEMA = "chxchx.agent-event"
SCHEMA_VERSION = 1

Emit = Callable[[dict[str, Any]], None]
# Providers that can pause a turn to ask the person (Claude: stream-json control requests, Codex: app-server,
# OpenCode: `opencode serve`).
LIVE_PROVIDERS = ("Claude", "Codex", "OpenCode")


@dataclass(frozen=True, slots=True)
class ChatTurnPlan:
    agent_id: str
    provider: str
    project_root: Path
    cwd: Path
    argv: list[str]
    mode: str
    session_id: str | None
    command: tuple[str, ...] = ()
    model: str | None = None
    effort: str | None = None
    images: tuple[Path, ...] = ()
    env: dict[str, str] = field(default_factory=dict)
    network: bool = False
    allow: tuple[str, ...] = ()
    # Claude only: ask the host while the turn runs (approvals and questions) instead of denying.
    live: bool = False
    # A one-off question (the Git assistant): no project context, no journal, no saved session, read-only.
    lean: bool = False


def load_prompt(prompt: str, prompt_file: Path | None) -> str:
    """The turn's prompt, from the option or from a file (a long prompt would not fit a command line)."""
    if prompt_file is not None:
        try:
            return prompt_file.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            raise WorkspaceOperationError(f"No pude leer el archivo del prompt: {exc}") from exc
    if not prompt.strip():
        raise WorkspaceOperationError("Indica el mensaje con --prompt o --prompt-file")
    return prompt


def plan_chat_turn(
    service: Any,
    agent_id: str,
    *,
    mode: str,
    session_id: str | None,
    model: str | None = None,
    effort: str | None = None,
    images: tuple[Path, ...] = (),
    lean: bool = False,
    network: bool = False,
    allow: tuple[str, ...] = (),
    live: bool = False,
) -> ChatTurnPlan:
    if lean:
        mode = "read-only"  # advice only: a lean turn must never be able to change files
    inspection = service.inspect()
    if not inspection.trusted:
        raise WorkspaceOperationError(
            "El proyecto no es confiable. Ejecuta `chxchx-tech workspace trust` antes de usar agentes."
        )
    if inspection.config is None:
        raise WorkspaceOperationError("No hay configuración de workspace")
    config = next((item for item in inspection.config.agents if item.id == agent_id), None)
    if config is None:
        raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
    if not isinstance(config.command, list) or not config.command:
        raise WorkspaceOperationError(f"El agente `{agent_id}` debe usar un comando por argumentos")
    provider = provider_for(config.command)
    if provider is None:
        raise WorkspaceOperationError("El chat visual solo está disponible para agentes Codex, Claude Code y OpenCode")
    if not executable_available(config.command[0]):
        raise WorkspaceOperationError(
            f"{provider} no está instalado o `{config.command[0]}` no está en el PATH. "
            f"Instálalo con: {INSTALL_HINTS[provider]}"
        )
    images = tuple(validated_image(path) for path in images)
    try:
        argv = turn_command(config.command, mode=mode, session_id=session_id, model=model, effort=effort,
                            images=[str(path) for path in images] if provider in IMAGE_ARGUMENT_PROVIDERS else (),
                            ephemeral=lean, network=network, allow=allow,
                            live=live and not lean and provider in LIVE_PROVIDERS)
    except ValueError as exc:
        raise WorkspaceOperationError(str(exc)) from exc
    argv[0] = resolve_executable(argv[0])
    root = service.project.root.resolve()
    live = live and not lean and provider in LIVE_PROVIDERS
    env = turn_environment(config.command, mode=mode, network=network, allow=allow)
    if live and provider == "OpenCode":
        env["OPENCODE_SERVER_PASSWORD"] = secrets.token_urlsafe(24)  # only this chat's host may drive the server
    return ChatTurnPlan(agent_id, provider, root, (root / config.cwd).resolve(), argv, mode, session_id,
                        tuple(config.command), model, effort, images, env, network, allow, live, lean)


def agent_options_payload(service: Any, agent_id: str) -> dict[str, Any]:
    """Read-only options for the chat panel; works before trust is granted."""
    inspection = service.inspect()
    agents = inspection.config.agents if inspection.config is not None else []
    config = next((item for item in agents if item.id == agent_id), None)
    if config is None or not isinstance(config.command, list) or not config.command:
        raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
    provider = provider_for(config.command)
    if provider is None:
        raise WorkspaceOperationError("El chat visual solo está disponible para agentes Codex, Claude Code y OpenCode")
    return {
        "schema": AGENT_OPTIONS_SCHEMA,
        "schema_version": BRIDGE_SCHEMA_VERSION,
        "agent": agent_id,
        **agent_options(provider, config.command[0]),
    }


def run_chat_turn(plan: ChatTurnPlan, prompt: str, emit: Emit, *, cancel_input: IO[str] | None = None) -> int:
    """Run the turn; a "cancel" line or EOF on `cancel_input` stops the agent."""
    emit({"type": "start", "agent": plan.agent_id, "provider": plan.provider, "mode": plan.mode})

    def notify(message: str, is_error: bool) -> None:
        emit({"type": "notice", "message": message, "error": is_error})

    argv = plan.argv
    staged: list[Path] = []
    if plan.images:
        staged = stage_images(plan.project_root, plan.images)
        if plan.provider in IMAGE_ARGUMENT_PROVIDERS:
            argv = turn_command(plan.command, mode=plan.mode, session_id=plan.session_id, model=plan.model,
                                effort=plan.effort, images=[str(path) for path in staged], ephemeral=plan.lean,
                                network=plan.network, allow=plan.allow, live=plan.live)
            argv[0] = resolve_executable(argv[0])
        else:
            listed = "\n".join(f"- {path.relative_to(plan.project_root).as_posix()}" for path in staged)
            prompt += f"\n\nImágenes adjuntas por el usuario (ábrelas con la herramienta Read antes de responder):\n{listed}"
    if plan.session_id is None and not plan.lean:
        recovered = prepare_new_chat(plan.project_root, notify)  # compacts the context files first
        context = new_chat_prompt(str(plan.project_root)) + inline_project_context(plan.project_root, provider=plan.provider) + recovered
        prompt = f"{context}\n\n{CHAT_PANEL_REQUEST_MARKER}\n{prompt}"
    started_at = datetime.now(timezone.utc)
    try:
        process = subprocess.Popen(
            argv,
            cwd=plan.cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            start_new_session=os.name != "nt",
            env={**os.environ, **plan.env} if plan.env else None,
        )
    except OSError as exc:
        emit({"type": "error", "message": f"No pude iniciar {argv[0]}: {exc}"})
        emit({"type": "done", "exit_code": 127, "cancelled": False, "session_id": plan.session_id})
        return 127

    stderr_tail: deque[str] = deque(maxlen=20)
    cancelled = threading.Event()
    assert process.stdin is not None
    live = _live_driver(plan, process, staged)
    threading.Thread(target=drain, args=(process.stderr, stderr_tail), daemon=True).start()
    if cancel_input is not None:
        threading.Thread(target=watch_cancel, args=(cancel_input, process, cancelled, live), daemon=True).start()
    try:
        if live is not None:
            live.start(prompt)  # stdin stays open: the answers to the agent's questions go through it
        else:
            process.stdin.write(prompt)
            process.stdin.close()
    except OSError:
        pass

    session_id = plan.session_id
    assert process.stdout is not None
    records = live.records(process.stdout) if isinstance(live, LiveOpenCode) else _json_lines(process.stdout)
    for record in records:
        if live is not None:
            controls = live.handle(record)
            if live.done:
                live.finish()  # the turn is over: let the agent exit
            if controls is not None:
                for event in controls:
                    if event["type"] == "session":
                        session_id = event["session_id"]
                    if not (cancelled.is_set() and event["type"] == "error"):  # stopping it is not a failure
                        emit(event)
                continue
        for event in normalize(plan.provider, record):
            if event["type"] == "session":
                session_id = event["session_id"]
            emit(event)
    exit_code = process.wait()
    if isinstance(live, LiveOpenCode) and live.stopped_by_us:
        exit_code = 0  # the server only ends because we stopped it after the turn
    if exit_code != 0 and not cancelled.is_set():
        detail = "\n".join(stderr_tail).strip()
        emit({"type": "error", "message": detail or f"{plan.provider} terminó con código {exit_code}"})
    if plan.provider == "Codex" and session_id:
        limits = codex_limits(session_id)
        if limits is not None:
            emit({"type": "limits", **limits})
    if plan.lean:
        # Nothing of a one-off question belongs in the memory or the session journal.
        if plan.provider == "OpenCode" and session_id and plan.command:
            delete_opencode_session(resolve_executable(plan.command[0]), session_id, plan.cwd)
    else:
        capture_after_session(plan.project_root, notify, provider=plan.provider, started_at=started_at,
                              opencode_executable=resolve_executable(plan.command[0]) if plan.provider == "OpenCode" and plan.command else None)
    emit({"type": "done", "exit_code": exit_code, "cancelled": cancelled.is_set(), "session_id": session_id})
    return exit_code


def _json_lines(stream: IO[str]) -> Iterator[dict[str, Any]]:
    for line in stream:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            yield record


def _live_driver(plan: ChatTurnPlan, process: subprocess.Popen[str], images: list[Path]) -> LiveClaude | LiveCodex | LiveOpenCode | None:
    if not plan.live:
        return None
    stdin = process.stdin
    assert stdin is not None
    if plan.provider == "OpenCode":
        return LiveOpenCode(process, password=plan.env["OPENCODE_SERVER_PASSWORD"], cwd=plan.cwd, session_id=plan.session_id,
                            agent=opencode_agent(Access(plan.mode, plan.network, plan.allow)), model=plan.model, effort=plan.effort,
                            images=images)
    if plan.provider == "Codex":
        return LiveCodex(stdin, cwd=plan.cwd, access=Access(plan.mode, plan.network, plan.allow), session_id=plan.session_id,
                         model=plan.model or configured_model(plan.command), effort=plan.effort, images=images)
    return LiveClaude(stdin)


def json_line_emitter(stream: IO[str] = sys.stdout) -> Emit:
    def emit(event: dict[str, Any]) -> None:
        payload = {"schema": SCHEMA, "schema_version": SCHEMA_VERSION, **event}
        stream.write(json.dumps(payload, ensure_ascii=False) + "\n")
        stream.flush()

    return emit
