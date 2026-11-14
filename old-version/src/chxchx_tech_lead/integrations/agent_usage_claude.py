"""Reader for Claude Code transcript context usage."""

from __future__ import annotations

import json
from pathlib import Path

from .agent_usage_common import modified_at, positive_int, same_path, tail_lines
from .agent_usage_models import LocalContextUsage


def read_claude_usage(path: Path, project_root: Path) -> LocalContextUsage | None:
    latest: LocalContextUsage | None = None
    try:
        with path.open("rb") as stream:
            for line in tail_lines(stream):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                cwd = record.get("cwd")
                if isinstance(cwd, str) and not same_path(Path(cwd), project_root):
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
                    if (value := positive_int(usage.get(key))) is not None
                )
                if not tokens:
                    continue
                window, percent = _context_fields(record)
                if percent is None and window:
                    percent = min(100.0, tokens * 100 / window)
                latest = LocalContextUsage("Claude", tokens, window, percent, modified_at(path))
    except (OSError, UnicodeError):
        return None
    return latest


def _context_fields(value: object) -> tuple[int | None, float | None]:
    if isinstance(value, dict):
        for key in ("context_window", "contextWindow"):
            context = value.get(key)
            if isinstance(context, dict):
                window = positive_int(context.get("context_window_size") or context.get("size"))
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
