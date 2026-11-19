"""The first message of a visual-chat session: the project context, what must continue, then the request."""

from __future__ import annotations

from pathlib import Path

from .agent_commands import CHAT_PANEL_REQUEST_MARKER, new_chat_prompt
from .chat_compaction import handoff_block
from .chat_context import Notifier, inline_project_context, prepare_new_chat


def first_message(
    project_root: Path,
    provider: str,
    request: str,
    notify: Notifier,
    *,
    handoff: str = "",
    recap: str = "",
    rust_context: str | None = None,
    rust_context_compacted_sections: int = 0,
) -> str:
    recovered = prepare_new_chat(
        project_root,
        notify,
        compact=rust_context is None,
        include_journal=rust_context is None,
    )
    if rust_context is None:
        context = new_chat_prompt(str(project_root)) + inline_project_context(project_root, provider=provider)
    else:
        context = rust_context
        if rust_context_compacted_sections:
            notify(
                f"Contexto compactado: {rust_context_compacted_sections} sección(es) antiguas movidas a .ai/history/.",
                False,
            )
    context += recovered
    context += handoff_block(handoff) if handoff else recap  # a hand-off is richer than a recap: it wins
    return f"{context}\n\n{CHAT_PANEL_REQUEST_MARKER}\n{request}"


def prepare_turn_prompt(
    project_root: Path,
    provider: str,
    request: str,
    notify: Notifier,
    *,
    session_id: str | None,
    lean: bool,
    housekeeping: bool,
    handoff: str,
    recap: str,
    rust_context: str | None,
    rust_context_compacted_sections: int,
) -> str:
    if session_id is not None or lean or housekeeping:
        return request
    return first_message(
        project_root,
        provider,
        request,
        notify,
        handoff=handoff,
        recap=recap,
        rust_context=rust_context,
        rust_context_compacted_sections=rust_context_compacted_sections,
    )
