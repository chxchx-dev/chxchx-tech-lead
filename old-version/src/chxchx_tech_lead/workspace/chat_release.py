"""Freeing a nearly full context: the agent writes a hand-off and the next message continues from it."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import replace
from typing import Any

from ..integrations.agent_catalog import resolve_executable
from ..integrations.agent_stream import turn_command, turn_environment
from .chat_compaction import HANDOFF_PROMPT, save_handoff

Emit = Callable[[dict[str, Any]], None]


def release_context(plan: Any, session_id: str, emit: Emit, run_turn: Callable[..., int]) -> None:
    """The context is nearly full: the agent writes a hand-off now (read-only, in its own session)."""
    try:
        argv = turn_command(plan.command, mode="read-only", session_id=session_id, model=plan.model, effort=plan.effort,
                            images=(), ephemeral=False, network=False, allow=(), live=False, opencode_major=plan.opencode_major)
    except ValueError:
        return
    argv[0] = resolve_executable(argv[0])
    quiet = replace(plan, argv=argv, mode="read-only", session_id=session_id, images=(), network=False, allow=(), live=False,
                    handoff="", recap="", env=turn_environment(plan.command, mode="read-only", network=False, allow=(),
                                                               opencode_major=plan.opencode_major))
    written: list[str] = []
    code = run_turn(quiet, HANDOFF_PROMPT, lambda event: written.append(event["text"]) if event["type"] == "message" else None,
                    housekeeping=True)
    summary = "\n\n".join(written).strip()
    if code != 0 or not summary:
        emit({"type": "notice", "message": "No pude liberar el contexto; sigo con la conversación actual.", "error": False})
        return
    save_handoff(plan.project_root, plan.provider, session_id, summary)
    emit({"type": "notice", "message": "Contexto liberado automáticamente: la conversación sigue con todo lo anterior.", "error": False})
