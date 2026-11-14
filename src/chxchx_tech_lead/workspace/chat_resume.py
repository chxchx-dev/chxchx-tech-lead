"""Resuming a restored chat: keep the provider session when it still exists, else rebuild the thread."""

from __future__ import annotations

from pathlib import Path

from ..integrations.agent_catalog import resolve_executable
from ..integrations.chat_history import list_conversations
from ..integrations.opencode_memory import run_json
from .chat_state import load_state, recap_block


def session_exists(project_root: Path, provider: str, session_id: str, executable: str) -> bool | None:
    """Whether the provider still holds the session. None: it cannot be told (assume it does)."""
    if provider == "OpenCode":
        sessions = run_json([resolve_executable(executable), "session", "list", "--max-count", "200", "--format", "json"], project_root)
        if not isinstance(sessions, list):
            return None
        return any(isinstance(item, dict) and item.get("id") == session_id for item in sessions)
    try:
        return any(item.session_id == session_id and item.provider == provider
                   for item in list_conversations(project_root, limit=2000))
    except OSError:
        return None


def resolve_resume(project_root: Path, provider: str, agent_id: str, session_id: str | None, executable: str) -> tuple[str | None, str]:
    """(session to resume, recap). A restored chat is checked once: when its session is gone the saved
    conversation travels in the first message of a new one, so nothing the person said is lost."""
    state = load_state(project_root, agent_id)
    if not session_id or state is None or state.get("session_id") != session_id or state.get("verified", True):
        return session_id, ""
    if session_exists(project_root, provider, session_id, executable) is False and state["messages"]:
        return None, recap_block(state)
    return session_id, ""
