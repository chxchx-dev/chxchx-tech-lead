"""The first message of a visual-chat session: the project context, what must continue, then the request."""

from __future__ import annotations

from pathlib import Path

from .agent_commands import CHAT_PANEL_REQUEST_MARKER, new_chat_prompt
from .chat_compaction import handoff_block
from .chat_context import Notifier, inline_project_context, prepare_new_chat


def first_message(project_root: Path, provider: str, request: str, notify: Notifier, *, handoff: str = "", recap: str = "") -> str:
    recovered = prepare_new_chat(project_root, notify)  # compacts the context files first
    context = new_chat_prompt(str(project_root)) + inline_project_context(project_root, provider=provider) + recovered
    context += handoff_block(handoff) if handoff else recap  # a hand-off is richer than a recap: it wins
    return f"{context}\n\n{CHAT_PANEL_REQUEST_MARKER}\n{request}"
