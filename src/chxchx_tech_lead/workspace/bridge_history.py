from __future__ import annotations

from pathlib import Path
from typing import Any

from ..integrations.chat_history import list_conversations
from .bridge_contracts import (
    CONVERSATION_SCHEMA,
    CONVERSATIONS_SCHEMA,
    ERRORS_SCHEMA,
    HANDOFF_SCHEMA,
    MEMORY_SCHEMA,
    SCHEMA_VERSION,
    ConversationPayload,
    ConversationsPayload,
    ErrorsPayload,
    HandoffPayload,
    MemoryPayload,
)
from .error_cache import error_cache_path, list_errors
from .memory_history import list_memory_notes


def handoff_payload(project_path: Path) -> HandoffPayload:
    root = project_path.expanduser().resolve(strict=True)
    target = root / ".ai" / "HANDOFF.md"
    resolved = target.resolve(strict=False)
    if not resolved.is_relative_to(root):
        raise ValueError("El handoff apunta fuera del proyecto.")
    if not target.is_file():
        return {
            "schema": HANDOFF_SCHEMA,
            "schema_version": SCHEMA_VERSION,
            "exists": False,
            "path": str(target),
            "content": "",
        }
    content = resolved.read_text(encoding="utf-8")

    def field(heading: str) -> str:
        lines = content.splitlines()
        marker = f"### {heading}"
        try:
            start = lines.index(marker) + 1
        except ValueError:
            return ""
        for line in lines[start:]:
            stripped = line.strip()
            if stripped.startswith("### "):
                break
            if stripped:
                return stripped[2:].strip() if stripped.startswith("- ") else ""
        return ""

    return {
        "schema": HANDOFF_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "exists": True,
        "path": str(resolved),
        "content": content,
        "summary": field("Último cambio"),
        "pending": field("Pendiente"),
        "validation": field("Validación"),
    }


def memory_payload(project_path: Path, query: str = "") -> MemoryPayload:
    root = project_path.expanduser().resolve(strict=True)
    memory_root = root / ".ai" / "memory"
    if memory_root.exists() and not memory_root.resolve().is_relative_to(root):
        raise ValueError("La memoria local apunta fuera del proyecto.")
    notes = list_memory_notes(root, query=query)
    return {
        "schema": MEMORY_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "project": str(root),
        "query": query,
        "notes": [
            {
                "path": str(note.path),
                "title": note.title,
                "modified_at": note.modified_at.isoformat(timespec="minutes"),
                "preview": note.preview,
                "content": note.content,
            }
            for note in notes
        ],
    }


def conversations_payload(project_path: Path, query: str = "") -> ConversationsPayload:
    root = project_path.expanduser().resolve(strict=True)
    conversations = list_conversations(root, query=query)
    return {
        "schema": CONVERSATIONS_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "project": str(root),
        "query": query,
        "conversations": [
            {
                "provider": item.provider,
                "session_id": item.session_id,
                "title": item.title,
                "modified_at": item.modified_at.isoformat(timespec="minutes"),
                "preview": item.preview,
                "transcript_path": str(item.transcript_path),
            }
            for item in conversations
        ],
    }


def conversation_payload(project_path: Path, provider: str, session_id: str) -> ConversationPayload:
    root = project_path.expanduser().resolve(strict=True)
    conversation = next(
        (
            item for item in list_conversations(root, limit=500)
            if item.provider.casefold() == provider.casefold()
            and item.session_id == session_id
        ),
        None,
    )
    if conversation is None:
        raise ValueError("No se encontró esa conversación para este proyecto.")
    return {
        "schema": CONVERSATION_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "provider": conversation.provider,
        "session_id": conversation.session_id,
        "title": conversation.title,
        "modified_at": conversation.modified_at.isoformat(timespec="minutes"),
        "transcript_path": str(conversation.transcript_path),
        "messages": [
            {"role": message.role, "text": message.text}
            for message in conversation.messages
        ],
    }


def errors_payload(project_path: Path) -> ErrorsPayload:
    root = project_path.expanduser().resolve(strict=True)
    errors = list_errors(project_path=root)
    return {
        "schema": ERRORS_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "project": str(root),
        "cache_path": str(error_cache_path()),
        "errors": [
            {
                "occurred_at": error.occurred_at,
                "project": error.project,
                "project_path": error.project_path,
                "operation": error.operation,
                "message": error.message,
            }
            for error in errors
        ],
    }
