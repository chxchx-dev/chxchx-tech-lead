"""Record a short summary of each agent session so the next chat can continue.

Continuity used to depend on the agent remembering to update the handoff. The
pane wrapper now writes what the session started with, the user's last
requests and the agent's closing message to `.ai/history/SESSIONS.md` when the
agent exits, and injects the latest entries into the next new-chat prompt.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from ..integrations.chat_history import Conversation, list_conversations
from ..integrations.explicit_memory import contains_secret, user_visible_text

JOURNAL = Path(".ai") / "history" / "SESSIONS.md"
MAX_ENTRIES = 30
_TITLE = "# Sesiones recientes de agentes\n\n"
_TOOL_PLACEHOLDER = re.compile(r"\[Herramienta: [^\]]*\]")
_REDACTED = "(omitido: posible secreto)"


@dataclass(frozen=True, slots=True)
class SessionSummary:
    provider: str
    session_id: str
    date: str
    opening: str
    requests: tuple[str, ...]
    outcome: str


def summarize_latest_session(
    project_root: Path, *, provider: str, started_at: datetime
) -> SessionSummary | None:
    """Summarize the provider's newest transcript if it was used after `started_at`."""
    for conversation in list_conversations(project_root, limit=10):
        if conversation.provider != provider:
            continue
        if conversation.modified_at.timestamp() < started_at.timestamp() - 5:
            return None
        return _summarize(conversation)
    return None


def record_session(project_root: Path, summary: SessionSummary) -> Path:
    """Insert or refresh the session entry at the top, keeping a bounded log."""
    path = project_root / JOURNAL
    marker = f"<!-- session:{summary.provider}:{summary.session_id} -->"
    entries = [entry for entry in _entries(path) if marker not in entry]
    entries.insert(0, _render(summary, marker))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(_TITLE + "".join(entries[:MAX_ENTRIES]), encoding="utf-8")
    return path


def recent_sessions_prompt(project_root: Path, *, count: int = 2) -> str:
    entries = _entries(project_root / JOURNAL)[:count]
    if not entries:
        return ""
    body = "".join(re.sub(r"<!-- session:[^>]*-->\n", "", entry) for entry in entries).strip()
    return (
        "\n\nResumen automático de las últimas sesiones de agentes en este proyecto "
        f"(`{JOURNAL.as_posix()}`). Úsalo para continuar; si contradice el handoff, "
        f"prioriza el archivo más reciente y verifica en el código.\n{body}"
    )


def _summarize(conversation: Conversation) -> SessionSummary | None:
    user = [text for text in (user_visible_text(m.text) for m in conversation.messages if m.role == "user") if text]
    assistant = [
        text
        for text in (
            " ".join(_TOOL_PLACEHOLDER.sub(" ", m.text).split())
            for m in conversation.messages
            if m.role == "assistant"
        )
        if text
    ]
    if not user:
        return None
    return SessionSummary(
        conversation.provider,
        conversation.session_id,
        conversation.modified_at.date().isoformat(),
        _safe(user[0], 240),
        tuple(_safe(text, 200) for text in user[1:][-3:]),
        _safe(assistant[-1], 600) if assistant else "",
    )


def _render(summary: SessionSummary, marker: str) -> str:
    lines = [
        f"## {summary.date} · {summary.provider} · `{summary.session_id}`",
        marker,
        "",
        f"- Inicio: {summary.opening}",
    ]
    if summary.requests:
        lines.append("- Últimos pedidos:")
        lines.extend(f"  - {request}" for request in summary.requests)
    if summary.outcome:
        lines.append(f"- Cierre del agente: {summary.outcome}")
    return "\n".join(lines) + "\n\n"


def _entries(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    parts = re.split(r"(?m)^(?=## )", text)
    return [part.rstrip("\n") + "\n\n" for part in parts if part.startswith("## ")]


def _safe(text: str, limit: int) -> str:
    if contains_secret(text):
        return _REDACTED
    return text if len(text) <= limit else text[: limit - 1].rstrip() + "…"
