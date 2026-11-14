"""Read local agent transcript usage without accessing account credentials."""

from __future__ import annotations

import json
import os
import hashlib
import tempfile
import time
from pathlib import Path

from .agent_usage_claude import read_claude_usage
from .agent_usage_codex import read_codex_usage
from .agent_usage_common import modified_at, number, positive_int
from .agent_usage_models import LiveAgentUsage, LocalContextUsage


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
        context_used_percent=number(context.get("used_percentage")),
        context_remaining_percent=number(context.get("remaining_percentage")),
        input_tokens=positive_int(context.get("total_input_tokens")),
        context_window=positive_int(context.get("context_window_size")),
        five_hour_used_percent=number(five_hour.get("used_percentage")),
        five_hour_resets_at=number(five_hour.get("resets_at")),
        seven_day_used_percent=number(seven_day.get("used_percentage")),
        seven_day_resets_at=number(seven_day.get("resets_at")),
        updated_at=number(data.get("updated_at")) or 0,
    )


def latest_context_usage(project_root: Path, provider: str) -> LocalContextUsage | None:
    """Return the newest supported context counters found in local transcripts.

    Provider transcript formats evolve. Missing or unrecognized fields are
    represented as unavailable rather than estimated from message length.
    """
    if provider.casefold() == "codex":
        paths = _codex_paths()
        reader = read_codex_usage
    elif provider.casefold() == "claude":
        paths = _claude_paths(project_root)
        reader = read_claude_usage
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
        return sorted(folder.glob(pattern), key=modified_at, reverse=True)[:100]
    except OSError:
        return []
