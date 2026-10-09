"""Recover only user messages that explicitly ask to remember something."""

from __future__ import annotations

import re
import hashlib
from dataclasses import dataclass
from pathlib import Path

from .chat_history import Conversation, list_conversations

_REMEMBER = re.compile(
    r"\b(?:recuerd\w*|record\w*|acuerd\w*|acord\w*|memor\w*|guard\w*|"
    r"remember\w*|don't forget|do not forget|save this|keep in mind|ten presente)\b",
    re.IGNORECASE,
)
_RECOVERY_QUESTION = re.compile(
    r"\b(?:recupera|recuerdame|recuérdame|dime|cual era|cuál era|"
    r"que te di|qué te di|que te dije|qué te dije|cual fue|cuál fue|"
    r"le puse que|le dije que|le pedi que|le pedí que|no se acuerda|"
    r"se debio|se debió|what was|what I gave you|"
    r"remind me|retrieve|recover)\b",
    re.IGNORECASE,
)
_SENSITIVE = re.compile(
    r"\b(?:password|contraseña|token|api[_ -]?key|secret|secreto|private key|"
    r"clave privada)\b|(?:sk-[A-Za-z0-9_-]{16,})",
    re.IGNORECASE,
)
_MAX_MESSAGE_CHARS = 1200
_MAX_MEMORIES = 8


@dataclass(frozen=True, slots=True)
class ExplicitMemory:
    provider: str
    session_id: str
    requested_at: str
    message: str


def recover_explicit_memories(project_root: Path, *, limit: int = 200) -> list[ExplicitMemory]:
    """Find bounded, project-scoped explicit memory requests in recent chats."""
    memories: list[ExplicitMemory] = []
    seen: set[str] = set()
    conversations = list_conversations(project_root, limit=limit)
    for conversation in conversations:
        for message in conversation.messages:
            text = " ".join(message.text.split())
            key = text.casefold()
            if message.role != "user" or key in seen or len(text) > _MAX_MESSAGE_CHARS:
                continue
            if not _REMEMBER.search(text) or _RECOVERY_QUESTION.search(text) or _SENSITIVE.search(text):
                continue
            seen.add(key)
            memories.append(
                ExplicitMemory(
                    conversation.provider,
                    conversation.session_id,
                    conversation.modified_at.date().isoformat(),
                    text,
                )
            )
            if len(memories) >= _MAX_MEMORIES:
                return memories
    return memories


def persist_explicit_memories(project_root: Path, memories: list[ExplicitMemory]) -> None:
    """Append recovered requests without duplicating existing project memory."""
    if not memories:
        return
    path = project_root / ".ai" / "memory" / "PROJECT_MEMORY.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    existing = path.read_text(encoding="utf-8") if path.exists() else ""
    additions: list[str] = []
    for memory in memories:
        digest = hashlib.sha256(memory.message.encode("utf-8")).hexdigest()[:12]
        marker = f"<!-- explicit-memory:{memory.provider}:{memory.session_id}:{digest} -->"
        if marker in existing:
            continue
        additions.append(
            f"\n### Petición explícita de memoria — {memory.requested_at}\n\n"
            f"{marker}\n"
            f"Origen: {memory.provider}, conversación `{memory.session_id}`.\n\n"
            f"> {memory.message}\n"
        )
        existing += marker
    if additions:
        with path.open("a", encoding="utf-8") as stream:
            stream.write("".join(additions))


def format_recovered_memories(memories: list[ExplicitMemory]) -> str:
    if not memories:
        return ""
    entries = "\n".join(f"- {item.message}" for item in memories)
    return (
        "\n\nMemoria explícita recuperada de chats anteriores del proyecto actual. "
        "Trata estos mensajes como solicitudes literales del usuario; responde a "
        "la solicitud actual usando el contenido exacto cuando corresponda.\n"
        f"{entries}"
    )
