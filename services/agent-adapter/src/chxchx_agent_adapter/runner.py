"""Runs one turn: starts the provider, drives its live protocol and reports what happens."""

from __future__ import annotations

import json
import os
import secrets
import threading
from collections import deque
from collections.abc import Callable, Iterator
from typing import IO, Any

from . import process as procs
from .codex_live import LiveCodex
from .contract import CONTRACT_VERSION, EVENT_SCHEMA, Request
from .opencode_live import LiveOpenCode
from .opencode_live2 import LiveOpenCodeV2
from .usage import codex_limits, opencode_window

Emit = Callable[[dict[str, Any]], None]


def json_lines(stream: IO[str]) -> Iterator[dict[str, Any]]:
    for line in stream:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            yield record


def json_line_emitter(stream: IO[str], lock: threading.Lock | None = None) -> Emit:
    guard = lock or threading.Lock()

    def emit(event: dict[str, Any]) -> None:
        payload = {"schema": EVENT_SCHEMA, "schema_version": CONTRACT_VERSION, **event}
        with guard:
            stream.write(json.dumps(payload, ensure_ascii=False) + "\n")
            stream.flush()

    return emit


def _driver(request: Request, child: Any, password: str) -> LiveCodex | LiveOpenCode | LiveOpenCodeV2:
    if request.provider == "Codex":
        return LiveCodex(child.stdin, cwd=request.cwd, settings=request.codex, session_id=request.session_id,
                         model=request.model, effort=request.effort, images=request.images)
    driver = LiveOpenCodeV2 if (request.opencode_major or 0) >= 2 else LiveOpenCode  # OpenCode 2 has another server API
    return driver(child, password=password, cwd=request.cwd, session_id=request.session_id, agent=request.opencode_agent,
                  model=request.model, effort=request.effort, images=request.images)


def run_turn(request: Request, emit: Emit, control: IO[str]) -> int:
    """Returns the exit code of the provider (0 for a turn that ended, or was stopped by us)."""
    password = secrets.token_urlsafe(24)  # only this turn's host may drive an OpenCode server
    env = {**os.environ, **request.env}
    if request.provider == "OpenCode":
        env["OPENCODE_SERVER_PASSWORD"] = password
    try:
        child = procs.start(request.argv, request.cwd, env)
    except OSError as error:
        emit({"type": "error", "message": f"No pude iniciar {request.argv[0]}: {error}"})
        emit({"type": "adapter_done", "exit_code": 127, "cancelled": False, "session_id": request.session_id})
        return 127

    stderr_tail: deque[str] = deque(maxlen=20)
    cancelled = threading.Event()
    live = _driver(request, child, password)
    threading.Thread(target=procs.drain, args=(child.stderr, stderr_tail), daemon=True).start()
    threading.Thread(target=procs.watch_control, args=(control, child, cancelled, live), daemon=True).start()
    try:
        live.start(request.prompt)  # stdin stays open: the answers to the agent's questions go through it
    except OSError:
        pass

    session_id = request.session_id
    window: int | None = None
    assert child.stdout is not None
    records = live.records(child.stdout) if isinstance(live, (LiveOpenCode, LiveOpenCodeV2)) else json_lines(child.stdout)
    for record in records:
        events = live.handle(record)
        if live.done:
            live.finish()  # the turn is over: let the agent exit
        for event in events or []:
            if event["type"] == "session":
                session_id = event["session_id"]
            if cancelled.is_set() and event["type"] == "error":
                continue  # stopping it is not a failure
            if event["type"] == "limits" and request.provider == "OpenCode" and event.get("context_window") is None:
                window = window or opencode_window(request.model or getattr(live, "model_seen", None))
                if window:
                    event = {**event, "context_window": window}
            emit(event)
    exit_code = child.wait()
    if isinstance(live, (LiveOpenCode, LiveOpenCodeV2)) and live.stopped_by_us:
        exit_code = 0  # the server only ends because we stopped it after the turn
    if exit_code != 0 and not cancelled.is_set():
        detail = "\n".join(stderr_tail).strip()
        emit({"type": "error", "message": detail or f"{request.provider} terminó con código {exit_code}"})
    if request.provider == "Codex" and session_id:
        limits = codex_limits(session_id)
        if limits is not None:
            emit({"type": "limits", **limits})
    emit({"type": "adapter_done", "exit_code": exit_code, "cancelled": cancelled.is_set(), "session_id": session_id})
    return exit_code
