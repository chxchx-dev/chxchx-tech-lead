"""Context shared by every way of opening an agent chat (terminal or visual)."""

from __future__ import annotations

from collections.abc import Callable
from datetime import datetime
from pathlib import Path

from ..integrations.explicit_memory import (
    format_recovered_memories,
    persist_explicit_memories,
    recover_explicit_memories,
)
from .context_compaction import auto_compact_enabled, compact_context
from .session_journal import recent_sessions_prompt, record_session, summarize_latest_session

# notify(message, is_error)
Notifier = Callable[[str, bool], None]


def prepare_new_chat(
    project_root: Path, notify: Notifier, *, opencode_executable: str | None = None
) -> str:
    """Compact context files and return the text to append to a new-chat prompt."""
    _compact(project_root, notify)
    extra = ""
    try:
        memories = recover_explicit_memories(project_root, **_memory_options(opencode_executable))
        persist_explicit_memories(project_root, memories)
        extra += format_recovered_memories(memories)
        if memories:
            notify(f"Memoria explícita recuperada: {len(memories)} solicitud(es) del proyecto.", False)
    except OSError as exc:
        notify(f"No pude recuperar la memoria local del proyecto: {exc}", True)
    try:
        extra += recent_sessions_prompt(project_root)
    except OSError as exc:
        notify(f"No pude leer el diario de sesiones: {exc}", True)
    return extra


def capture_after_session(
    project_root: Path,
    notify: Notifier,
    *,
    provider: str | None,
    started_at: datetime,
    opencode_executable: str | None = None,
) -> None:
    """Persist explicit memories and the session summary once an agent exits."""
    try:
        memories = recover_explicit_memories(project_root, **_memory_options(opencode_executable))
        persist_explicit_memories(project_root, memories)
    except OSError as exc:
        notify(f"No pude guardar la memoria local del proyecto: {exc}", True)
    if provider is None:
        return
    try:
        journal_options = {"opencode_executable": opencode_executable} if provider == "OpenCode" and opencode_executable else {}
        summary = summarize_latest_session(project_root, provider=provider, started_at=started_at, **journal_options)
        if summary is not None:
            record_session(project_root, summary)
    except OSError as exc:
        notify(f"No pude guardar el resumen de la sesión: {exc}", True)


def _compact(project_root: Path, notify: Notifier) -> None:
    if not auto_compact_enabled(project_root):
        return
    try:
        result = compact_context(project_root)
    except OSError as exc:
        notify(f"No pude compactar el contexto del proyecto: {exc}", True)
        return
    moved = sum(len(item.moved) for item in result.files)
    if moved:
        notify(f"Contexto compactado: {moved} sección(es) antiguas movidas a .ai/history/.", False)


def _memory_options(opencode_executable: str | None) -> dict[str, str]:
    return {"opencode_executable": opencode_executable} if opencode_executable else {}
