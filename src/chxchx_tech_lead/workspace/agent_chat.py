"""Run one visual-chat turn of a configured agent and stream normalized events.

Studio's chat panel starts `chxchx-tech agent chat` per user message. This use
case keeps the authority in Python: trust, context, memory capture and the
session journal are the same ones the terminal agent pane uses.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
from collections import deque
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import IO, Any

from ..integrations.agent_stream import normalize, provider_for, turn_command
from .agent_commands import CHAT_PANEL_REQUEST_MARKER, new_chat_prompt
from .chat_context import capture_after_session, prepare_new_chat
from .process_runtime import terminate_posix_process_group, terminate_windows_process_tree
from .service import WorkspaceOperationError

SCHEMA = "chxchx.agent-event"
SCHEMA_VERSION = 1
_STOP_TIMEOUT_SECONDS = 3.0

Emit = Callable[[dict[str, Any]], None]


@dataclass(frozen=True, slots=True)
class ChatTurnPlan:
    agent_id: str
    provider: str
    project_root: Path
    cwd: Path
    argv: list[str]
    mode: str
    session_id: str | None


def plan_chat_turn(service: Any, agent_id: str, *, mode: str, session_id: str | None) -> ChatTurnPlan:
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
        raise WorkspaceOperationError("El chat visual solo está disponible para agentes Codex y Claude Code")
    try:
        argv = turn_command(config.command, mode=mode, session_id=session_id)
    except ValueError as exc:
        raise WorkspaceOperationError(str(exc)) from exc
    root = service.project.root.resolve()
    return ChatTurnPlan(agent_id, provider, root, (root / config.cwd).resolve(), argv, mode, session_id)


def run_chat_turn(plan: ChatTurnPlan, prompt: str, emit: Emit, *, cancel_input: IO[str] | None = None) -> int:
    """Run the turn; a "cancel" line or EOF on `cancel_input` stops the agent."""
    emit({"type": "start", "agent": plan.agent_id, "provider": plan.provider, "mode": plan.mode})

    def notify(message: str, is_error: bool) -> None:
        emit({"type": "notice", "message": message, "error": is_error})

    if plan.session_id is None:
        context = new_chat_prompt(str(plan.project_root)) + prepare_new_chat(plan.project_root, notify)
        prompt = f"{context}\n\n{CHAT_PANEL_REQUEST_MARKER}\n{prompt}"
    started_at = datetime.now(timezone.utc)
    try:
        process = subprocess.Popen(
            plan.argv,
            cwd=plan.cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            start_new_session=os.name != "nt",
        )
    except OSError as exc:
        emit({"type": "error", "message": f"No pude iniciar {plan.argv[0]}: {exc}"})
        emit({"type": "done", "exit_code": 127, "cancelled": False, "session_id": plan.session_id})
        return 127

    stderr_tail: deque[str] = deque(maxlen=20)
    cancelled = threading.Event()
    threading.Thread(target=_drain, args=(process.stderr, stderr_tail), daemon=True).start()
    if cancel_input is not None:
        threading.Thread(target=_watch_cancel, args=(cancel_input, process, cancelled), daemon=True).start()
    try:
        assert process.stdin is not None
        process.stdin.write(prompt)
        process.stdin.close()
    except OSError:
        pass

    session_id = plan.session_id
    assert process.stdout is not None
    for line in process.stdout:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        for event in normalize(plan.provider, record):
            if event["type"] == "session":
                session_id = event["session_id"]
            emit(event)
    exit_code = process.wait()
    if exit_code != 0 and not cancelled.is_set():
        detail = "\n".join(stderr_tail).strip()
        emit({"type": "error", "message": detail or f"{plan.provider} terminó con código {exit_code}"})
    capture_after_session(plan.project_root, notify, provider=plan.provider, started_at=started_at)
    emit({"type": "done", "exit_code": exit_code, "cancelled": cancelled.is_set(), "session_id": session_id})
    return exit_code


def json_line_emitter(stream: IO[str] = sys.stdout) -> Emit:
    def emit(event: dict[str, Any]) -> None:
        payload = {"schema": SCHEMA, "schema_version": SCHEMA_VERSION, **event}
        stream.write(json.dumps(payload, ensure_ascii=False) + "\n")
        stream.flush()

    return emit


def _drain(stream: IO[str] | None, tail: deque[str]) -> None:
    if stream is None:
        return
    for line in stream:
        if line.strip():
            tail.append(line.rstrip())


def _watch_cancel(source: IO[str], process: subprocess.Popen[str], cancelled: threading.Event) -> None:
    for line in source:
        if line.strip() == "cancel":
            break
    # EOF also lands here: the caller went away, so never leave the agent running.
    if process.poll() is not None:
        return
    cancelled.set()
    try:
        if os.name == "nt":
            terminate_windows_process_tree(process.pid, process, timeout_seconds=_STOP_TIMEOUT_SECONDS)
        else:
            terminate_posix_process_group(process.pid, process, timeout_seconds=_STOP_TIMEOUT_SECONDS)
    except (OSError, subprocess.TimeoutExpired):
        process.kill()
