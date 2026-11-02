"""Recover only user messages that explicitly ask to remember something."""

from __future__ import annotations

import os
import re
import hashlib
import tempfile
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from ..core.paths import ensure_home
from .chat_history import Conversation, list_conversations

# Directives must open a sentence (or follow "y"/"por favor") so that nouns such
# as "memoria RAM" or everyday verbs such as "guarda el archivo" are not captured.
_SENTENCE_START = r"(?:^|(?<=[.!?¡¿;:,])\s*|\b(?:y|and|también|also|por favor|please),?\s+)"
_REMEMBER = re.compile(
    rf"{_SENTENCE_START}(?:recu[eé]rda(?:lo|la|los|las)?|recordá|acu[eé]rdate|memor[ií]za\w*|"
    r"ten presente|no (?:te )?olvides|remember|memorize|keep in mind|don'?t forget|do not forget)\b"
    r"|\b(?:quiero|necesito|me gustaría) que (?:te acuerdes|recuerdes|memorices|tengas presente|no olvides)\b"
    r"|\b(?:i want you to|i need you to) (?:remember|memorize|keep in mind)\b"
    r"|\b(?:gu[aá]rd\w*|anot\w*|save|store) (?:\S+ ){0,3}?(?:en|in|to) (?:la |tu |your )?memor(?:ia|y)\b",
    re.IGNORECASE,
)
_RECOVERY_QUESTION = re.compile(
    r"\b(?:recupera|recuerdame|recuérdame|dime|cual era|cuál era|"
    r"que te di|qué te di|que te dije|qué te dije|cual fue|cuál fue|"
    r"le puse que|le dije que|le pedi que|le pedí que|no se acuerda|"
    r"no (?:lo |la )?recuerda|no (?:lo |la )?record[oó]|se olvid[oó]|olvid[oó]|"
    r"se debio|se debió|what was|what I gave you|forgot|doesn't remember|does not remember|"
    r"remind me|retrieve|recover)\b",
    re.IGNORECASE,
)
# Agents inject context (system reminders, IDE selections, command output) into
# user turns; those blocks are not words the user typed.
_INJECTED_BLOCK = re.compile(r"<([A-Za-z][\w-]*)\b[^>]*>.*?</\1>", re.DOTALL)
_MEMORY_ENTRY = re.compile(
    r"\n### Petición explícita de memoria — [^\n]*\n\n<!-- explicit-memory:[^\n]*?-->\n.*?(?=\n#{1,6} |\Z)",
    re.DOTALL,
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


def recover_explicit_memories(
    project_root: Path,
    *,
    limit: int = 200,
    opencode_executable: str | None = None,
) -> list[ExplicitMemory]:
    """Find bounded, project-scoped explicit memory requests across agents."""
    memories: list[ExplicitMemory] = []
    seen: set[str] = set()

    from .opencode_memory import explicit_user_messages

    for session_id, requested_at, message in explicit_user_messages(
        project_root, executable=opencode_executable
    ):
        text = user_visible_text(message)
        key = text.casefold()
        if not _is_explicit_request(text, key, seen):
            continue
        seen.add(key)
        memories.append(ExplicitMemory("OpenCode", session_id, requested_at, text))
        if len(memories) >= _MAX_MEMORIES:
            return memories

    for conversation in list_conversations(project_root, limit=limit):
        for message in conversation.messages:
            text = user_visible_text(message.text)
            key = text.casefold()
            if message.role != "user" or not _is_explicit_request(text, key, seen):
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


def user_visible_text(message: str) -> str:
    """Collapse whitespace and drop blocks the agent injected into a user turn."""
    return " ".join(_INJECTED_BLOCK.sub(" ", message).split())


def contains_secret(text: str) -> bool:
    return _SENSITIVE.search(text) is not None


def _is_explicit_request(text: str, key: str, seen: set[str]) -> bool:
    return (
        bool(text)
        and key not in seen
        and len(text) <= _MAX_MESSAGE_CHARS
        and _REMEMBER.search(text) is not None
        and _RECOVERY_QUESTION.search(text) is None
        and not contains_secret(text)
    )


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
        # The same request repeated in another chat is still one memory.
        if f":{digest} -->" in existing:
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


@dataclass(frozen=True, slots=True)
class MemoryPruneResult:
    path: Path
    removed: tuple[str, ...]
    backup: Path | None


def prune_explicit_memories(project_root: Path, *, dry_run: bool = False) -> MemoryPruneResult:
    """Drop captured entries that the current detector rejects or that repeat.

    Only sections written by ``persist_explicit_memories`` are evaluated; notes
    the user wrote by hand are preserved verbatim. A backup is kept outside the
    project before rewriting the file.
    """
    root = project_root.resolve()
    path = root / ".ai" / "memory" / "PROJECT_MEMORY.md"
    if not path.is_file():
        return MemoryPruneResult(path, (), None)
    content = path.read_text(encoding="utf-8")
    removed: list[str] = []
    seen: set[str] = set()

    def keep_or_drop(match: re.Match[str]) -> str:
        quoted = re.search(r"^> (.*)$", match.group(0), re.MULTILINE)
        text = user_visible_text(quoted.group(1)) if quoted else ""
        key = text.casefold()
        if _is_explicit_request(text, key, seen):
            seen.add(key)
            return match.group(0)
        removed.append(text)
        return ""

    pruned = _MEMORY_ENTRY.sub(keep_or_drop, content)
    if not removed or dry_run:
        return MemoryPruneResult(path, tuple(removed), None)
    backup = _backup_memory_file(root, path)
    _atomic_write(path, pruned)
    return MemoryPruneResult(path, tuple(removed), backup)


def _backup_memory_file(root: Path, path: Path) -> Path:
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    key = hashlib.sha256(str(root).encode("utf-8")).hexdigest()[:12]
    backup = ensure_home() / "backups" / f"{root.name}-{key}" / "memory" / stamp / path.name
    backup.parent.mkdir(parents=True, exist_ok=False)
    backup.write_bytes(path.read_bytes())
    return backup


def _atomic_write(path: Path, content: str) -> None:
    descriptor, temporary = tempfile.mkstemp(prefix=".chxchx-memory-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
        os.replace(temporary, path)
    except OSError:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


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
