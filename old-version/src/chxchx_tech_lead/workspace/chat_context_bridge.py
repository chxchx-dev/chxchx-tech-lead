"""Versioned file contract for context prepared by the Rust Studio host."""

from __future__ import annotations

import json
from pathlib import Path

from .service import WorkspaceOperationError

SCHEMA = "chxchx.studio-chat-context"
SCHEMA_VERSION = 1


def load_rust_context(context_file: Path | None) -> tuple[str | None, int]:
    if context_file is None:
        return None, 0
    try:
        payload = json.loads(context_file.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise WorkspaceOperationError(f"No pude leer el contexto preparado por Studio: {exc}") from exc
    if not isinstance(payload, dict):
        raise WorkspaceOperationError("El archivo de contexto de Studio no cumple el contrato v1")
    context = payload.get("context")
    compacted = payload.get("compacted_sections")
    if (
        payload.get("schema") != SCHEMA
        or payload.get("schema_version") != SCHEMA_VERSION
        or not isinstance(context, str)
        or type(compacted) is not int
        or compacted < 0
    ):
        raise WorkspaceOperationError("El archivo de contexto de Studio no cumple el contrato v1")
    return context, compacted
