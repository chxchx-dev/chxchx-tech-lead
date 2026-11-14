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

from ..integrations.agent_opencode_live import LiveOpenCode
from ..integrations.agent_opencode_live2 import LiveOpenCodeV2
from ..integrations.agent_opencode_turn import opencode_major
from ..integrations.executable_search import with_search_path
from ..integrations.agent_catalog import INSTALL_HINTS, agent_options, codex_limits, executable_available, resolve_executable
from ..integrations.agent_stream import IMAGE_ARGUMENT_PROVIDERS, normalize, provider_for, turn_command, turn_environment
from .bridge_contracts import AGENT_OPTIONS_SCHEMA, SCHEMA_VERSION as BRIDGE_SCHEMA_VERSION
from .chat_compaction import UsageWatch, consume_handoff, load_handoff, needs_release
from .chat_drivers import json_lines, live_driver
from .chat_options import agent_options_payload  # noqa: F401  (public name kept)
from .chat_attachments import remove_staged_images, stage_images, validated_image
from .chat_process import delete_opencode_session, drain, watch_cancel
from .chat_context import capture_after_session
from .chat_first_message import first_message
from .chat_release import release_context
from .chat_resume import resolve_resume
from .chat_state import begin_turn, end_turn
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
    # The context was freed after the last turn: the first message of the new session carries the hand-off.
    handoff: str = ""
    continued_from: str | None = None
    # Free the context automatically when it passes 90%.
    auto_release: bool = True
    # OpenCode 2 has another local API and permission format (see agent_opencode_turn); None when unknown or not OpenCode.
    opencode_major: int | None = None
    # The session of a restored chat is gone: the saved conversation travels in the first message of a new one.
    recap: str = ""


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
    auto_release: bool = True,
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
    root = service.project.root.resolve()
    handoff = "" if lean else load_handoff(root, provider, session_id)
    major = opencode_major(resolve_executable(config.command[0])) if provider == "OpenCode" else None
    can_live = live and not lean and provider in LIVE_PROVIDERS
    continued_from, session_id = (session_id, None) if handoff else (None, session_id)
    recap = ""
    if not lean and not handoff:
        session_id, recap = resolve_resume(root, provider, agent_id, session_id, config.command[0])
    try:
        argv = turn_command(config.command, mode=mode, session_id=session_id, model=model, effort=effort,
                            images=[str(path) for path in images] if provider in IMAGE_ARGUMENT_PROVIDERS else (),
                            ephemeral=lean, network=network, allow=allow, live=can_live, opencode_major=major)
    except ValueError as exc:
        raise WorkspaceOperationError(str(exc)) from exc
    argv[0] = resolve_executable(argv[0])
    live = can_live
    env = turn_environment(config.command, mode=mode, network=network, allow=allow, opencode_major=major)
    if live and provider == "OpenCode":
        env["OPENCODE_SERVER_PASSWORD"] = secrets.token_urlsafe(24)  # only this chat's host may drive the server
    return ChatTurnPlan(agent_id, provider, root, (root / config.cwd).resolve(), argv, mode, session_id,
                        tuple(config.command), model, effort, images, env, network, allow, live, lean, handoff, continued_from,
                        auto_release, major, recap)


def run_chat_turn(plan: ChatTurnPlan, prompt: str, emit: Emit, *, cancel_input: IO[str] | None = None,
                  housekeeping: bool = False) -> int:
    """Run the turn; a "cancel" line or EOF on `cancel_input` stops the agent.

    `housekeeping` is an internal turn (the hand-off): no start/done events, no context, journal or memory.
    """
    if not housekeeping:
        emit({"type": "start", "agent": plan.agent_id, "provider": plan.provider, "mode": plan.mode})
    watch = UsageWatch(plan.provider, plan.model)
    user_text, replies = prompt, []
    remember = not plan.lean and not housekeeping  # the visual chat keeps its conversation across restarts

    def notify(message: str, is_error: bool) -> None:
        emit({"type": "notice", "message": message, "error": is_error})

    argv = plan.argv
    staged: list[Path] = []
    if plan.images:
        staged = stage_images(plan.project_root, plan.images)
        if plan.provider in IMAGE_ARGUMENT_PROVIDERS:
            argv = turn_command(plan.command, mode=plan.mode, session_id=plan.session_id, model=plan.model,
                                effort=plan.effort, images=[str(path) for path in staged], ephemeral=plan.lean,
                                network=plan.network, allow=plan.allow, live=plan.live, opencode_major=plan.opencode_major)
            argv[0] = resolve_executable(argv[0])
        else:
            listed = "\n".join(f"- {path.relative_to(plan.project_root).as_posix()}" for path in staged)
            prompt += f"\n\nImágenes adjuntas por el usuario (ábrelas con la herramienta Read antes de responder):\n{listed}"
    if plan.session_id is None and not plan.lean and not housekeeping:
        prompt = first_message(plan.project_root, plan.provider, prompt, notify, handoff=plan.handoff, recap=plan.recap)
    if remember:
        _remember(begin_turn, plan, plan.session_id or plan.continued_from, user_text)
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
            env=with_search_path({**os.environ, **plan.env}),
        )
    except OSError as exc:
        emit({"type": "error", "message": f"No pude iniciar {argv[0]}: {exc}"})
        if plan.lean:
            remove_staged_images(plan.project_root, staged)
        emit({"type": "done", "exit_code": 127, "cancelled": False, "session_id": plan.session_id})
        return 127

    stderr_tail: deque[str] = deque(maxlen=20)
    cancelled = threading.Event()
    assert process.stdin is not None
    live = live_driver(plan, process, staged)
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

    def deliver(event: dict[str, Any]) -> None:
        event = watch.observe(event, getattr(live, "model_seen", None))
        if event["type"] == "message":
            replies.append(str(event.get("text", "")))
        emit(event)

    session_id = plan.session_id
    assert process.stdout is not None
    records = live.records(process.stdout) if isinstance(live, (LiveOpenCode, LiveOpenCodeV2)) else json_lines(process.stdout)
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
                        deliver(event)
                continue
        for event in normalize(plan.provider, record):
            if event["type"] == "session":
                session_id = event["session_id"]
            deliver(event)
    exit_code = process.wait()
    if plan.lean:
        remove_staged_images(plan.project_root, staged)
    if isinstance(live, (LiveOpenCode, LiveOpenCodeV2)) and live.stopped_by_us:
        exit_code = 0  # the server only ends because we stopped it after the turn
    if exit_code != 0 and not cancelled.is_set():
        detail = "\n".join(stderr_tail).strip()
        emit({"type": "error", "message": detail or f"{plan.provider} terminó con código {exit_code}"})
    if housekeeping:
        return exit_code
    if plan.provider == "Codex" and session_id:
        limits = codex_limits(session_id)
        if limits is not None:
            emit(watch.observe({"type": "limits", **limits}))
    if plan.handoff and exit_code == 0 and session_id and session_id != plan.continued_from:
        consume_handoff(plan.project_root, plan.provider, plan.continued_from)  # the new session has it now
    if plan.auto_release and not plan.lean and exit_code == 0 and not cancelled.is_set() and session_id and needs_release(*watch.usage):
        release_context(plan, session_id, emit, run_chat_turn)
    if plan.lean:
        # Nothing of a one-off question belongs in the memory or the session journal.
        if plan.provider == "OpenCode" and session_id and plan.command:
            delete_opencode_session(resolve_executable(plan.command[0]), session_id, plan.cwd)
    else:
        capture_after_session(plan.project_root, notify, provider=plan.provider, started_at=started_at,
                              opencode_executable=resolve_executable(plan.command[0]) if plan.provider == "OpenCode" and plan.command else None)
    if remember:
        _remember(end_turn, plan, session_id, replies)
    emit({"type": "done", "exit_code": exit_code, "cancelled": cancelled.is_set(), "session_id": session_id})
    return exit_code


def _remember(write: Callable[..., None], plan: ChatTurnPlan, session_id: str | None, payload: Any) -> None:
    try:
        write(plan.project_root, plan.agent_id, plan.provider, session_id, payload)
    except OSError:
        pass  # a read-only project must not stop the chat; it just will not be restored


def json_line_emitter(stream: IO[str] = sys.stdout) -> Emit:
    def emit(event: dict[str, Any]) -> None:
        payload = {"schema": SCHEMA, "schema_version": SCHEMA_VERSION, **event}
        stream.write(json.dumps(payload, ensure_ascii=False) + "\n")
        stream.flush()

    return emit
