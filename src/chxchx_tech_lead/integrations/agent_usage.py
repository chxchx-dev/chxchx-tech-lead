"""Read local agent transcript usage without accessing account credentials."""

from __future__ import annotations

import json
import os
import hashlib
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class LocalContextUsage:
    provider: str
    input_tokens: int | None = None
    context_window: int | None = None
    used_percent: float | None = None
    updated_at: float = 0.0


@dataclass(frozen=True, slots=True)
class LiveAgentUsage:
    context_used_percent: float | None = None
    context_remaining_percent: float | None = None
    input_tokens: int | None = None
    context_window: int | None = None
    five_hour_used_percent: float | None = None
    five_hour_resets_at: float | None = None
    seven_day_used_percent: float | None = None
    seven_day_resets_at: float | None = None
    updated_at: float = 0.0


def usage_snapshot_path(project_root: Path, agent_id: str) -> Path:
    root_key = hashlib.sha256(str(project_root.expanduser().resolve()).encode()).hexdigest()[:16]
    agent_key = hashlib.sha256(agent_id.encode()).hexdigest()[:12]
    return Path(tempfile.gettempdir()) / "chxchx-tech-lead" / f"{root_key}-{agent_key}.json"


def read_live_usage(project_root: Path, agent_id: str) -> LiveAgentUsage | None:
    path = usage_snapshot_path(project_root, agent_id)
    try:
        if path.stat().st_mtime < time.time() - 90:
            return None
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(data, dict):
        return None
    context = data.get("context_window") if isinstance(data.get("context_window"), dict) else {}
    limits = data.get("rate_limits") if isinstance(data.get("rate_limits"), dict) else {}
    five_hour = limits.get("five_hour") if isinstance(limits.get("five_hour"), dict) else {}
    seven_day = limits.get("seven_day") if isinstance(limits.get("seven_day"), dict) else {}
    return LiveAgentUsage(
        context_used_percent=_number(context.get("used_percentage")),
        context_remaining_percent=_number(context.get("remaining_percentage")),
        input_tokens=_positive_int(context.get("total_input_tokens")),
        context_window=_positive_int(context.get("context_window_size")),
        five_hour_used_percent=_number(five_hour.get("used_percentage")),
        five_hour_resets_at=_number(five_hour.get("resets_at")),
        seven_day_used_percent=_number(seven_day.get("used_percentage")),
        seven_day_resets_at=_number(seven_day.get("resets_at")),
        updated_at=_number(data.get("updated_at")) or 0,
    )


def latest_context_usage(project_root: Path, provider: str) -> LocalContextUsage | None:
    """Return the newest supported context counters found in local transcripts.

    Provider transcript formats evolve. Missing or unrecognized fields are
    represented as unavailable rather than estimated from message length.
    """
    if provider.casefold() == "codex":
        paths = _codex_paths()
        reader = _codex_usage
    elif provider.casefold() == "claude":
        paths = _claude_paths(project_root)
        reader = _claude_usage
    else:
        return None

    root = project_root.expanduser().resolve()
    newest: LocalContextUsage | None = None
    for path in paths:
        if not path.is_file():
            continue
        result = reader(path, root)
        if result:
            newest = result
            break
    return newest


def _codex_paths() -> list[Path]:
    home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser()
    return _recent(home / "sessions", "rollout-*.jsonl")


def _claude_paths(project_root: Path) -> list[Path]:
    home = Path(os.environ.get("CLAUDE_CONFIG_DIR", Path.home() / ".claude")).expanduser()
    folder = str(project_root).replace("\\", "/").replace(":", "-").replace("/", "-")
    return _recent(home / "projects" / folder, "*.jsonl")


def _recent(folder: Path, pattern: str) -> list[Path]:
    if not folder.is_dir():
        return []
    try:
        return sorted(folder.glob(pattern), key=_mtime, reverse=True)[:100]
    except OSError:
        return []


def _codex_usage(path: Path, project_root: Path) -> LocalContextUsage | None:
    latest: LocalContextUsage | None = None
    same_project = False
    try:
        with path.open("rb") as stream:
            for _ in range(100):
                line = stream.readline()
                if not line:
                    break
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                payload = record.get("payload")
                if record.get("type") == "session_meta" and isinstance(payload, dict):
                    raw_cwd = payload.get("cwd")
                    same_project = isinstance(raw_cwd, str) and _same_path(Path(raw_cwd), project_root)
                    break
            if not same_project:
                return None
            for line in _tail_lines(stream):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                payload = record.get("payload")
                if not same_project or record.get("type") != "event_msg" or not isinstance(payload, dict):
                    continue
                if payload.get("type") != "token_count":
                    continue
                info = payload.get("info")
                if not isinstance(info, dict):
                    continue
                usage = info.get("last_token_usage")
                tokens = _positive_int(usage.get("input_tokens")) if isinstance(usage, dict) else None
                window = _positive_int(info.get("model_context_window"))
                percent = min(100.0, tokens * 100 / window) if tokens and window else None
                latest = LocalContextUsage("Codex", tokens, window, percent, _mtime(path))
    except (OSError, UnicodeError):
        return None
    return latest if same_project else None


def _claude_usage(path: Path, project_root: Path) -> LocalContextUsage | None:
    latest: LocalContextUsage | None = None
    try:
        with path.open("rb") as stream:
            for line in _tail_lines(stream):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                cwd = record.get("cwd")
                if isinstance(cwd, str) and not _same_path(Path(cwd), project_root):
                    continue
                message = record.get("message")
                if not isinstance(message, dict) or message.get("role") != "assistant":
                    continue
                usage = message.get("usage")
                if not isinstance(usage, dict):
                    continue
                tokens = sum(
                    value
                    for key in ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens")
                    if (value := _positive_int(usage.get(key))) is not None
                )
                if not tokens:
                    continue
                window, percent = _context_fields(record)
                if percent is None and window:
                    percent = min(100.0, tokens * 100 / window)
                latest = LocalContextUsage("Claude", tokens, window, percent, _mtime(path))
    except (OSError, UnicodeError):
        return None
    return latest


def _context_fields(value: object) -> tuple[int | None, float | None]:
    if isinstance(value, dict):
        for key in ("context_window", "contextWindow"):
            context = value.get(key)
            if isinstance(context, dict):
                window = _positive_int(context.get("context_window_size") or context.get("size"))
                percent_value = context.get("used_percentage") or context.get("usedPercentage")
                try:
                    percent = float(percent_value) if percent_value is not None else None
                except (TypeError, ValueError):
                    percent = None
                return window, percent
        for child in value.values():
            window, percent = _context_fields(child)
            if window is not None or percent is not None:
                return window, percent
    elif isinstance(value, list):
        for child in value:
            window, percent = _context_fields(child)
            if window is not None or percent is not None:
                return window, percent
    return None, None


def _positive_int(value: object) -> int | None:
    try:
        number = int(value)
    except (TypeError, ValueError):
        return None
    return number if number > 0 else None


def _number(value: object) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if number >= 0 else None


def _tail_lines(stream) -> list[bytes]:
    stream.seek(0, os.SEEK_END)
    stream.seek(max(0, stream.tell() - 1_000_000))
    if stream.tell():
        stream.readline()
    return stream.readlines()


def _mtime(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def _same_path(path: Path, expected: Path) -> bool:
    try:
        return path.expanduser().resolve() == expected
    except OSError:
        return False
