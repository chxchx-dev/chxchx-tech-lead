from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from ..core.paths import home_dir

_MAX_ENTRIES = 200
_MAX_MESSAGE_CHARS = 2400


@dataclass(frozen=True, slots=True)
class CachedError:
    occurred_at: str
    project: str
    project_path: str
    operation: str
    message: str


def error_cache_path() -> Path:
    return home_dir() / "cache" / "errors.jsonl"


def record_error(
    *, project: str, project_path: Path, operation: str, message: str
) -> None:
    """Append an error summary to a small bounded local cache."""
    path = error_cache_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    entries = _read_raw(path)
    entries.append(
        {
            "occurred_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "project": project[:120],
            "project_path": str(project_path)[:500],
            "operation": operation[:120],
            "message": message[:_MAX_MESSAGE_CHARS],
        }
    )
    _write_entries(path, entries[-_MAX_ENTRIES:])


def list_errors(*, project_path: Path | None = None, limit: int = 200) -> list[CachedError]:
    """Read recent cached errors, optionally scoped to one project."""
    entries = _read_raw(error_cache_path())
    if project_path is not None:
        wanted = str(project_path.expanduser().resolve())
        entries = [item for item in entries if item.get("project_path") == wanted]
    result: list[CachedError] = []
    for item in entries[-max(0, limit):]:
        try:
            result.append(
                CachedError(
                    occurred_at=str(item["occurred_at"]),
                    project=str(item["project"]),
                    project_path=str(item["project_path"]),
                    operation=str(item["operation"]),
                    message=str(item["message"]),
                )
            )
        except (KeyError, TypeError):
            continue
    return list(reversed(result))


def _read_raw(path: Path) -> list[dict[str, str]]:
    if not path.is_file():
        return []
    try:
        with path.open("r", encoding="utf-8") as stream:
            values = []
            for line in stream:
                if not line.strip():
                    continue
                try:
                    value = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(value, dict):
                    values.append(value)
    except (OSError, UnicodeError):
        return []
    return values[-_MAX_ENTRIES:]


def _write_entries(path: Path, entries: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8") as stream:
        for entry in entries:
            stream.write(json.dumps(entry, ensure_ascii=False) + "\n")
