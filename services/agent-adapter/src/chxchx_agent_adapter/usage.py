"""Context usage that the providers do not always report themselves."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any


def positive_int(value: object) -> int | None:
    try:
        number = int(value)  # type: ignore[call-overload]
    except (TypeError, ValueError):
        return None
    return number if number > 0 else None


def _tail_lines(stream: Any) -> list[bytes]:
    stream.seek(0, os.SEEK_END)
    stream.seek(max(0, stream.tell() - 1_000_000))
    if stream.tell():
        stream.readline()
    return stream.readlines()


def _window(raw: Any) -> dict[str, Any] | None:
    if not isinstance(raw, dict) or not isinstance(raw.get("used_percent"), (int, float)):
        return None
    return {
        "used_percent": float(raw["used_percent"]),
        "window_minutes": positive_int(raw.get("window_minutes")),
        "resets_at": positive_int(raw.get("resets_at")),
    }


def codex_limits(session_id: str) -> dict[str, Any] | None:
    """Latest context and plan usage recorded in the rollout of a Codex thread."""
    sessions = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser() / "sessions"
    try:
        rollouts = sorted(sessions.rglob(f"rollout-*{session_id}.jsonl"), key=lambda path: path.stat().st_mtime)
    except OSError:
        return None
    if not rollouts:
        return None
    latest: dict[str, Any] | None = None
    try:
        with rollouts[-1].open("rb") as stream:
            for line in _tail_lines(stream):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                payload = record.get("payload") if isinstance(record, dict) else None
                if isinstance(payload, dict) and payload.get("type") == "token_count":
                    latest = payload
    except (OSError, UnicodeError):
        return None
    if latest is None:
        return None
    info = latest.get("info") if isinstance(latest.get("info"), dict) else {}
    last = info.get("last_token_usage") if isinstance(info.get("last_token_usage"), dict) else {}
    limits = latest.get("rate_limits") if isinstance(latest.get("rate_limits"), dict) else {}
    return {
        "context_tokens": positive_int(last.get("input_tokens")),
        "context_window": positive_int(info.get("model_context_window")),
        "primary": _window(limits.get("primary")),
        "secondary": _window(limits.get("secondary")),
        "plan": limits.get("plan_type") if isinstance(limits.get("plan_type"), str) else None,
    }


def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None


def _limit(data: Any, provider: str, model: str) -> int | None:
    try:
        value = data[provider]["models"][model]["limit"]["context"]
    except (KeyError, TypeError):
        return None
    return value if isinstance(value, int) and value > 0 else None


def _config_home() -> Path:
    return Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")).expanduser()


def opencode_model(configured: str | None = None) -> str | None:
    """`provider/model` OpenCode will use: the one asked for, or the default of its configuration."""
    if configured and "/" in configured:
        return configured
    for name in ("opencode.json", "opencode.jsonc"):
        config = _read_json(_config_home() / "opencode" / name)
        if isinstance(config, dict) and isinstance(config.get("model"), str) and "/" in config["model"]:
            return config["model"]
    return None


def opencode_window(model: str | None) -> int | None:
    """Context window of an OpenCode model: from the person's provider settings, else OpenCode's own models cache."""
    model = opencode_model(model)
    if not model:
        return None
    provider, _, name = model.partition("/")
    for file in ("opencode.json", "opencode.jsonc"):
        config = _read_json(_config_home() / "opencode" / file)
        found = _limit(config.get("provider") if isinstance(config, dict) else None, provider, name)
        if found:
            return found
    cache_home = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")).expanduser()
    return _limit(_read_json(cache_home / "opencode" / "models.json"), provider, name)
