"""Wire protocol for normalized chat events crossing the CLI and Studio boundary."""

from __future__ import annotations

import json
import os
import sys
from collections.abc import Callable
from typing import IO, Any

SCHEMA = "chxchx.agent-event"
SCHEMA_VERSION = 1
Emit = Callable[[dict[str, Any]], None]


def json_line_emitter(stream: IO[str] = sys.stdout) -> Emit:
    def emit(event: dict[str, Any]) -> None:
        payload = {"schema": SCHEMA, "schema_version": SCHEMA_VERSION, **event}
        stream.write(json.dumps(payload, ensure_ascii=False) + "\n")
        stream.flush()

    return emit


def rust_normalizes(provider: str, *, live: bool) -> bool:
    """The Rust host consumes Claude stream-json and non-live provider JSONL records."""
    return os.environ.get("CHXCHX_RUST_AGENT_STREAM") == "1" and (
        provider == "Claude" or (provider in {"Codex", "OpenCode"} and not live)
    )
