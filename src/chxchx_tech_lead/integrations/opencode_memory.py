"""Read project-scoped OpenCode sessions through its supported CLI export."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


_SESSION_ID = re.compile(r"[A-Za-z0-9_-]{1,128}\Z")
_MAX_SESSIONS = 10
_COMMAND_TIMEOUT_SECONDS = 5


def explicit_user_messages(
    project_root: Path,
    *,
    executable: str | None = None,
    limit: int = _MAX_SESSIONS,
) -> list[tuple[str, str, str]]:
    """Return (session ID, date, user message) tuples for this project only."""
    root = project_root.expanduser().resolve()
    binary = executable or shutil.which("opencode")
    if not binary or not root.is_dir():
        return []

    count = min(max(limit, 0), _MAX_SESSIONS)
    if count == 0:
        return []
    sessions = _run_json([binary, "session", "list", "--max-count", str(count), "--format", "json"], root)
    if not isinstance(sessions, list):
        return []

    found: list[tuple[str, str, str]] = []
    for session in sessions[:count]:
        if not isinstance(session, dict):
            continue
        session_id = session.get("id")
        directory = session.get("directory")
        if not isinstance(session_id, str) or not _SESSION_ID.fullmatch(session_id):
            continue
        if not isinstance(directory, str) or not _belongs_to_project(directory, root):
            continue

        exported = _run_json([binary, "export", session_id, "--sanitize"], root)
        if not isinstance(exported, dict):
            continue
        info = exported.get("info")
        if isinstance(info, dict):
            exported_directory = info.get("directory")
            if isinstance(exported_directory, str) and not _belongs_to_project(exported_directory, root):
                continue
        date = _session_date(session)
        for message in exported.get("messages", []):
            if not isinstance(message, dict):
                continue
            message_info = message.get("info")
            role = message_info.get("role") if isinstance(message_info, dict) else None
            if role != "user":
                continue
            parts = message.get("parts", [])
            text = "\n".join(
                part["text"]
                for part in parts
                if isinstance(part, dict) and part.get("type") == "text" and isinstance(part.get("text"), str)
            ) if isinstance(parts, list) else ""
            if text.strip():
                found.append((session_id, date, text))
    return found


def _run_json(command: list[str], cwd: Path) -> Any:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=_COMMAND_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return None


def _belongs_to_project(directory: str, project_root: Path) -> bool:
    try:
        return Path(directory).expanduser().resolve() == project_root
    except (OSError, RuntimeError):
        return False


def _session_date(session: dict[str, Any]) -> str:
    time_data = session.get("time")
    updated = time_data.get("updated") if isinstance(time_data, dict) else None
    if isinstance(updated, (int, float)):
        timestamp = updated / 1000 if updated > 10_000_000_000 else updated
        try:
            return datetime.fromtimestamp(timestamp, tz=timezone.utc).date().isoformat()
        except (OverflowError, OSError, ValueError):
            pass
    if isinstance(updated, str):
        try:
            return datetime.fromisoformat(updated.replace("Z", "+00:00")).date().isoformat()
        except ValueError:
            pass
    return datetime.now(timezone.utc).date().isoformat()
