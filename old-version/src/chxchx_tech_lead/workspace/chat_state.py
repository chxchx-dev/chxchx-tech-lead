"""What a visual chat remembers when Studio is closed: its session and its conversation.

Each (project, agent) keeps `.ai/chat-state/<agent>.json`: the provider session id and the last messages (text
only; tool cards are not kept). Studio restores them when the chat opens, so closing a tab or the whole app does
not lose the conversation. The file is written when a turn starts (so an interrupted turn still shows the
person's message) and when it ends. Messages that look like they hold a secret are never stored.
"""

from __future__ import annotations

import json
import os
import re
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from ..integrations.explicit_memory import contains_secret

SCHEMA = "chxchx.chat-state"
SCHEMA_VERSION = 1
STATE_DIR = Path(".ai") / "chat-state"
MAX_MESSAGES = 80
MAX_MESSAGE_CHARS = 8_000
_OMITTED = "(omitido: posible secreto)"
_RECAP_MESSAGES = 24
_RECAP_CHARS = 14_000


def state_path(project_root: Path, agent_id: str) -> Path:
    safe = re.sub(r"[^A-Za-z0-9_.-]", "_", agent_id) or "agent"
    return project_root / STATE_DIR / f"{safe}.json"


def load_state(project_root: Path, agent_id: str) -> dict[str, Any] | None:
    try:
        data = json.loads(state_path(project_root, agent_id).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(data, dict) or not isinstance(data.get("messages"), list):
        return None
    messages = [m for m in data["messages"] if isinstance(m, dict) and m.get("role") in {"user", "assistant"}
                and isinstance(m.get("text"), str)]
    return {**data, "messages": messages}


def _write(project_root: Path, agent_id: str, state: dict[str, Any]) -> None:
    target = state_path(project_root, agent_id)
    target.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            json.dump(state, stream, ensure_ascii=False, indent=1)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, target)  # a crash never leaves half a file
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _clean(text: str) -> str:
    text = text.strip()
    if contains_secret(text):
        return _OMITTED
    return text[:MAX_MESSAGE_CHARS] + ("…" if len(text) > MAX_MESSAGE_CHARS else "")


def _record(project_root: Path, agent_id: str, provider: str, session_id: str | None, add: list[tuple[str, str]],
            verified: bool) -> None:
    state = load_state(project_root, agent_id) or {"messages": []}
    messages = [*state["messages"], *({"role": role, "text": _clean(text)} for role, text in add if text.strip())]
    _write(project_root, agent_id, {
        "schema": SCHEMA, "schema_version": SCHEMA_VERSION, "agent": agent_id, "provider": provider,
        "session_id": session_id or state.get("session_id") or "",
        "updated": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "verified": verified, "messages": messages[-MAX_MESSAGES:],
    })


def begin_turn(project_root: Path, agent_id: str, provider: str, session_id: str | None, user_text: str) -> None:
    """The person's message is saved before the agent answers: closing mid-turn does not lose it."""
    _record(project_root, agent_id, provider, session_id, [("user", user_text)], verified=True)


def end_turn(project_root: Path, agent_id: str, provider: str, session_id: str | None, replies: list[str]) -> None:
    _record(project_root, agent_id, provider, session_id, [("assistant", text) for text in replies], verified=True)


def reset_state(project_root: Path, agent_id: str) -> None:
    """A new chat starts from nothing."""
    try:
        state_path(project_root, agent_id).unlink()
    except OSError:
        pass


def mark_unverified(project_root: Path, agent_id: str) -> None:
    """Restoring does not know whether the provider still has the session; the next turn checks once."""
    state = load_state(project_root, agent_id)
    if state is not None and state.get("verified", True):
        _write(project_root, agent_id, {**state, "verified": False})


def state_payload(project_root: Path, agent_id: str, *, restoring: bool = False) -> dict[str, Any]:
    """What Studio needs to repaint the chat. `restoring` flags the session to be checked by the next turn."""
    state = load_state(project_root, agent_id)
    if restoring:
        mark_unverified(project_root, agent_id)
    return {"schema": SCHEMA, "schema_version": SCHEMA_VERSION, "agent": agent_id,
            "provider": (state or {}).get("provider", ""), "session_id": (state or {}).get("session_id", ""),
            "updated": (state or {}).get("updated", ""), "messages": (state or {}).get("messages", [])}


def _clip(text: str, limit: int) -> str:
    """Long messages keep their start and their end (the conclusion is usually the last lines)."""
    if len(text) <= limit:
        return text
    tail = min(300, limit // 3)
    return f"{text[: limit - tail - 3]}…\n…{text[-tail:]}"


def recap_block(state: dict[str, Any]) -> str:
    """The saved conversation as context for a new session, when the provider no longer has the old one."""
    lines: list[str] = []
    budget = _RECAP_CHARS
    for message in reversed(state["messages"][-_RECAP_MESSAGES:]):
        who = "Persona" if message["role"] == "user" else "Agente"
        text = _clip(message["text"], max(400, min(2_000, budget)))
        budget -= len(text)
        lines.append(f"{who}: {text}")
        if budget <= 0:
            break
    body = "\n\n".join(reversed(lines))
    return (
        "\n\nEsta conversación continúa de una sesión anterior que el agente ya no conserva. Retómala desde el "
        "historial de abajo como si nada hubiera cambiado: no lo menciones, no pidas confirmación ni repitas lo ya hecho, "
        "y responde al último mensaje de la persona.\n"
        f'<archivo-de-contexto ruta="conversación anterior">\n{body}\n</archivo-de-contexto>'
    )
