"""Models, effort levels, availability and usage limits of agent CLIs.

Codex publishes its model catalog (with the effort levels each model accepts)
in `~/.codex/models_cache.json` and its plan limits in the session rollouts.
Claude Code documents model aliases and `--effort` levels; its cost and
context usage arrive on the stream-json `result` message instead. OpenCode
lists `provider/model` ids with `opencode models` and reports tokens and cost
on each `step_finish` event.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tomllib
from pathlib import Path
from typing import Any

from .agent_usage_common import positive_int, tail_lines
from .executable_search import find_executable, with_search_path

INSTALL_HINTS = {
    "Codex": "npm install -g @openai/codex",
    "Claude": "curl -fsSL https://claude.ai/install.sh | bash",
    "OpenCode": "npm install -g opencode-ai",
}
_CODEX_FALLBACK_EFFORTS = ["low", "medium", "high", "xhigh", "max"]
_CLAUDE_MODELS = ["sonnet", "opus", "haiku", "fable", "opusplan", "sonnet[1m]", "opus[1m]"]
# Versioned ids the `--model` flag accepts, newest first; the aliases above follow the latest one.
_CLAUDE_VERSIONS = [
    "claude-fable-5-2", "claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5",
    "claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-1",
]
_CLAUDE_MODEL_ID = re.compile(r'"model":\s*"(claude-[a-z0-9.\-]+(?:\[1m\])?)"')
_CLAUDE_EFFORTS = ["low", "medium", "high", "xhigh", "max", "ultracode"]


def executable_available(executable: str) -> bool:
    if "/" in executable or "\\" in executable:
        return Path(executable).expanduser().exists()
    return find_executable(executable) is not None


def resolve_executable(executable: str) -> str:
    """Full path of a bare command, so Windows `.cmd` shims (npm installs) can be launched.

    `CreateProcess` only looks for `.exe` when given a bare name, while `shutil.which`
    honours PATHEXT. Paths and unknown commands are returned unchanged.
    """
    if "/" in executable or "\\" in executable:
        return executable
    return find_executable(executable) or executable


def agent_options(provider: str, executable: str) -> dict[str, Any]:
    """Selectable models/efforts plus the defaults the agent would use."""
    options: dict[str, Any] = {
        "provider": provider,
        "available": executable_available(executable),
        "install_hint": INSTALL_HINTS.get(provider, ""),
        "default_model": "",
        "default_effort": "",
        "models": [],
    }
    if provider == "Codex":
        options.update(_codex_options())
    elif provider == "Claude":
        options.update(_claude_options())
    elif provider == "OpenCode":
        options.update(_opencode_options(executable))
    return options


def codex_limits(session_id: str) -> dict[str, Any] | None:
    """Latest context and plan usage recorded in the rollout of a Codex thread."""
    sessions = _codex_home() / "sessions"
    try:
        rollouts = sorted(sessions.rglob(f"rollout-*{session_id}.jsonl"), key=lambda p: p.stat().st_mtime)
    except OSError:
        return None
    if not rollouts:
        return None
    latest: dict[str, Any] | None = None
    try:
        with rollouts[-1].open("rb") as stream:
            for line in tail_lines(stream):
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


def _codex_options() -> dict[str, Any]:
    home = _codex_home()
    config = _read_toml(home / "config.toml")
    models: list[dict[str, Any]] = []
    try:
        cache = json.loads((home / "models_cache.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        cache = {}
    for model in cache.get("models", []) if isinstance(cache, dict) else []:
        if not isinstance(model, dict) or model.get("visibility") != "list" or not isinstance(model.get("slug"), str):
            continue
        levels = model.get("supported_reasoning_levels")
        efforts = [
            level["effort"] for level in levels if isinstance(level, dict) and isinstance(level.get("effort"), str)
        ] if isinstance(levels, list) else []
        models.append({
            "id": model["slug"],
            "label": str(model.get("display_name") or model["slug"]),
            "efforts": efforts or _CODEX_FALLBACK_EFFORTS,
            "default_effort": str(model.get("default_reasoning_level") or ""),
        })
    return {
        "default_model": str(config.get("model") or ""),
        "default_effort": str(config.get("model_reasoning_effort") or ""),
        "models": models,
    }


def _claude_options() -> dict[str, Any]:
    config_dir = Path(os.environ.get("CLAUDE_CONFIG_DIR", Path.home() / ".claude")).expanduser()
    try:
        settings = json.loads((config_dir / "settings.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        settings = {}
    settings = settings if isinstance(settings, dict) else {}
    default = str(settings.get("model") or "")
    ids = [*_CLAUDE_MODELS, *_CLAUDE_VERSIONS, *_claude_extra_models(config_dir, settings), default]
    seen: list[str] = []
    for model in ids:
        if model and model not in seen:
            seen.append(model)
    return {
        "default_model": default,
        "default_effort": str(settings.get("effortLevel") or ""),
        "models": [
            {"id": model, "label": model, "efforts": _CLAUDE_EFFORTS, "default_effort": ""} for model in seen
        ],
    }


def _claude_extra_models(config_dir: Path, settings: dict[str, Any]) -> list[str]:
    """Models this machine already uses: `availableModels`, `ANTHROPIC_*_MODEL` and the latest transcripts."""
    found = [str(m) for m in settings.get("availableModels", []) if isinstance(m, str)]
    found += [v for k, v in os.environ.items() if k.startswith("ANTHROPIC_DEFAULT_") and k.endswith("_MODEL") and v]
    try:
        files = sorted((config_dir / "projects").glob("*/*.jsonl"), key=lambda f: f.stat().st_mtime, reverse=True)[:25]
        for file in files:
            with file.open(encoding="utf-8", errors="ignore") as stream:
                found += _CLAUDE_MODEL_ID.findall(stream.read(2_000_000))
    except OSError:
        pass
    return sorted(set(found))


def _opencode_options(executable: str) -> dict[str, Any]:
    """Default model from `opencode.json` plus the `provider/model` ids `opencode models` lists.

    Reasoning variants (`--variant`) differ per model and the CLI does not
    publish them, so no effort levels are offered: the model's own default applies.
    """
    config_home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")).expanduser()
    config: dict[str, Any] = {}
    for name in ("opencode.json", "opencode.jsonc"):
        try:
            loaded = json.loads((config_home / "opencode" / name).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            continue
        if isinstance(loaded, dict):
            config = loaded
            break
    default = str(config.get("model") or "")
    configured = [
        f"{provider}/{model}"
        for provider, spec in (config.get("provider") or {}).items()
        if isinstance(spec, dict)
        for model in (spec.get("models") or {})
    ]
    ids: list[str] = []
    for model in [*_opencode_models(executable), *configured, default]:
        if "/" in model and model not in ids:
            ids.append(model)
    return {
        "default_model": default,
        "default_effort": "",
        "models": [{"id": model, "label": model, "efforts": [], "default_effort": ""} for model in ids],
    }


def _opencode_models(executable: str) -> list[str]:
    if not executable_available(executable):
        return []
    try:
        result = subprocess.run([resolve_executable(executable), "models"], capture_output=True, text=True, encoding="utf-8",
                                errors="replace", timeout=20, check=False, env=with_search_path())
    except (OSError, subprocess.TimeoutExpired):
        return []
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if "/" in line.strip() and " " not in line.strip()]


def _window(raw: Any) -> dict[str, Any] | None:
    if not isinstance(raw, dict) or not isinstance(raw.get("used_percent"), (int, float)):
        return None
    return {
        "used_percent": float(raw["used_percent"]),
        "window_minutes": positive_int(raw.get("window_minutes")),
        "resets_at": positive_int(raw.get("resets_at")),
    }


def _codex_home() -> Path:
    return Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser()


def _read_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as stream:
            data = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError):
        return {}
    return data if isinstance(data, dict) else {}
