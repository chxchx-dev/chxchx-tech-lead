"""Read the newest OpenCode session of a project through its supported CLI export.

OpenCode keeps sessions in its own database, so unlike the Codex and Claude
transcripts they are not listed with the file-based history: this reader runs
only when a session summary is needed and loads a single session.
"""

from __future__ import annotations

import shutil
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .chat_history import Conversation, ConversationMessage
from .opencode_memory import belongs_to_project, run_json

_CANDIDATES = 5


def latest_opencode_conversation(project_root: Path, *, executable: str | None = None) -> Conversation | None:
    root = project_root.expanduser().resolve()
    binary = executable or shutil.which("opencode")
    if not binary or not root.is_dir():
        return None
    sessions = run_json([binary, "session", "list", "--max-count", str(_CANDIDATES), "--format", "json"], root)
    if not isinstance(sessions, list):
        return None
    for session in sessions:
        if not isinstance(session, dict) or not isinstance(session.get("id"), str):
            continue
        directory = session.get("directory")
        if not isinstance(directory, str) or not belongs_to_project(directory, root):
            continue
        exported = run_json([binary, "export", session["id"], "--sanitize"], root)
        return _conversation(session, exported, root)
    return None


def _conversation(session: dict[str, Any], exported: Any, root: Path) -> Conversation | None:
    if not isinstance(exported, dict) or not isinstance(exported.get("messages"), list):
        return None
    messages: list[ConversationMessage] = []
    for message in exported["messages"]:
        info = message.get("info") if isinstance(message, dict) else None
        role = info.get("role") if isinstance(info, dict) else None
        parts = message.get("parts") if isinstance(message, dict) else None
        if role not in {"user", "assistant"} or not isinstance(parts, list):
            continue
        text = "\n".join(
            part["text"] for part in parts
            if isinstance(part, dict) and part.get("type") == "text" and isinstance(part.get("text"), str)
        ).strip()
        if text:
            messages.append(ConversationMessage(role, text))
    if not messages:
        return None
    title = next((message.text for message in messages if message.role == "user"), "Chat OpenCode")
    return Conversation("OpenCode", session["id"], " ".join(title.split())[:90], _updated_at(session), root, root,
                        tuple(messages))


def _updated_at(session: dict[str, Any]) -> datetime:
    time_data = session.get("time")
    updated = time_data.get("updated") if isinstance(time_data, dict) else None
    if isinstance(updated, (int, float)):
        try:
            return datetime.fromtimestamp(updated / 1000 if updated > 10_000_000_000 else updated, tz=timezone.utc)
        except (OverflowError, OSError, ValueError):
            pass
    return datetime.now(timezone.utc)
