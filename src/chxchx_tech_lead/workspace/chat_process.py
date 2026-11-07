"""Helpers around the agent process of a visual-chat turn: reading stderr, listening to Studio, cleanup."""

from __future__ import annotations

import json
import os
import subprocess
import threading
from collections import deque
from pathlib import Path
from typing import IO, Any

from .process_runtime import terminate_posix_process_group, terminate_windows_process_tree

_STOP_TIMEOUT_SECONDS = 3.0


def delete_opencode_session(executable: str, session_id: str, cwd: Path) -> None:
    """OpenCode cannot skip saving a session, so a lean turn removes it afterwards (best effort)."""
    try:
        subprocess.run([executable, "session", "delete", session_id], cwd=cwd, capture_output=True, timeout=20, check=False)
    except (OSError, subprocess.TimeoutExpired):
        pass


def drain(stream: IO[str] | None, tail: deque[str]) -> None:
    if stream is None:
        return
    for line in stream:
        if line.strip():
            tail.append(line.rstrip())


def watch_cancel(source: IO[str], process: subprocess.Popen[str], cancelled: threading.Event,
                  live: Any = None) -> None:
    for line in source:
        text = line.strip()
        if text == "cancel":
            break
        if live is not None and text.startswith("{"):
            try:
                message = json.loads(text)
            except json.JSONDecodeError:
                continue
            if isinstance(message, dict) and message.get("type") == "answer":
                live.answer(message)
    # EOF also lands here: the caller went away, so never leave the agent running.
    if process.poll() is not None:
        return
    cancelled.set()
    try:
        if os.name == "nt":
            terminate_windows_process_tree(process.pid, process, timeout_seconds=_STOP_TIMEOUT_SECONDS)
        else:
            terminate_posix_process_group(process.pid, process, timeout_seconds=_STOP_TIMEOUT_SECONDS)
    except (OSError, subprocess.TimeoutExpired):
        process.kill()
