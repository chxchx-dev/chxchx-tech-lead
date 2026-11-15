"""Reader for Codex session transcript context usage."""

from __future__ import annotations

import json
from pathlib import Path

from .agent_usage_common import modified_at, positive_int, same_path, tail_lines
from .agent_usage_models import LocalContextUsage


def read_codex_usage(path: Path, project_root: Path) -> LocalContextUsage | None:
    same_project = False
    latest: LocalContextUsage | None = None
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
                    same_project = isinstance(raw_cwd, str) and same_path(Path(raw_cwd), project_root)
                    break
            if not same_project:
                return None
            for line in tail_lines(stream):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                payload = record.get("payload")
                if record.get("type") != "event_msg" or not isinstance(payload, dict):
                    continue
                if payload.get("type") != "token_count":
                    continue
                info = payload.get("info")
                if not isinstance(info, dict):
                    continue
                usage = info.get("last_token_usage")
                tokens = positive_int(usage.get("input_tokens")) if isinstance(usage, dict) else None
                window = positive_int(info.get("model_context_window"))
                percent = min(100.0, tokens * 100 / window) if tokens and window else None
                latest = LocalContextUsage("Codex", tokens, window, percent, modified_at(path))
    except (OSError, UnicodeError):
        return None
    return latest
