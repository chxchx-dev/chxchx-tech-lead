"""Structured (non-TTY) turns for Codex, Claude Code and OpenCode, normalized for Studio.

Each CLI offers an official machine-readable mode: `codex exec --json`,
`claude -p --output-format stream-json` and `opencode run --format json`.
Each user turn runs one process that
resumes the previous session, and the prompt goes through stdin so Windows
`.cmd` shims never have to quote free text. Every provider event is mapped to
the `chxchx.agent-event` v1 shapes below; unknown events are dropped.

- session:   {"type": "session", "session_id": str}
- message:   {"type": "message", "role": "assistant", "text": str}
- reasoning: {"type": "reasoning", "text": str}
- tool:      {"type": "tool", "id": str, "kind": str, "title": str,
              "status": "running" | "completed" | "failed", "output": str}
- usage:     {"type": "usage", "input_tokens": int, "output_tokens": int, "cost_usd": float | None}
- limits:    {"type": "limits", "context_tokens": int | None, "context_window": int | None,
              "primary": window | None, "secondary": window | None, "plan": str | None}
             where window = {"used_percent": float, "window_minutes": int, "resets_at": epoch}
- error:     {"type": "error", "message": str}
"""

from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from .agent_stream_opencode import normalize_opencode

PROVIDERS = {"codex": "Codex", "claude": "Claude", "opencode": "OpenCode"}
# Providers that take attachments as CLI arguments; Claude reads them from the prompt.
IMAGE_ARGUMENT_PROVIDERS = frozenset({"Codex", "OpenCode"})
MODES = ("read-only", "workspace-write")
_MAX_OUTPUT_CHARS = 4000
_CODEX_SANDBOX = {"read-only": "read-only", "workspace-write": "workspace-write"}
_CLAUDE_PERMISSION = {"read-only": "plan", "workspace-write": "acceptEdits"}
_OPENCODE_AGENT = {"read-only": "plan", "workspace-write": "build"}
# The built-in `plan` agent still allows bash, so read-only also denies it explicitly.
_OPENCODE_READ_ONLY_PERMISSION = {"edit": "deny", "bash": "deny"}

Event = dict[str, Any]


def provider_for(command: Sequence[str]) -> str | None:
    executable = Path(command[0].replace("\\", "/")).name.casefold() if command else ""
    for suffix in (".exe", ".cmd", ".bat"):
        executable = executable.removesuffix(suffix)
    return PROVIDERS.get(executable)


def turn_command(
    command: Sequence[str],
    *,
    mode: str,
    session_id: str | None,
    model: str | None = None,
    effort: str | None = None,
    images: Sequence[str] = (),
) -> list[str]:
    """Build the structured-turn argv; the prompt is written to stdin.

    Claude receives images as paths inside the prompt (see `agent_chat`), so
    only Codex and OpenCode take them as arguments.
    """
    if mode not in MODES:
        raise ValueError(f"Modo no soportado: {mode}")
    provider = provider_for(command)
    executable, configured = command[0], list(command[1:])
    if provider == "Codex":
        argv = [executable, "exec", *configured, "--json", "-s", _CODEX_SANDBOX[mode]]
        if model:
            argv += ["-m", model]
        if effort:
            argv += ["-c", f'model_reasoning_effort="{effort}"']
        for image in images:
            argv += ["--image", image]
        # A flag after --image ends its value list before the positional prompt.
        argv.append("--skip-git-repo-check")
        return [*argv, "resume", session_id, "-"] if session_id else [*argv, "-"]
    if provider == "Claude":
        argv = [executable, *configured, "-p", "--output-format", "stream-json", "--verbose",
                "--permission-mode", _CLAUDE_PERMISSION[mode]]
        if model:
            argv += ["--model", model]
        if effort:
            argv += ["--effort", effort]
        return [*argv, "--resume", session_id] if session_id else argv
    if provider == "OpenCode":
        argv = [executable, "run", *configured, "--format", "json", "--agent", _OPENCODE_AGENT[mode]]
        if session_id:
            argv += ["--session", session_id]
        if model:
            argv += ["--model", model]
        if effort:
            argv += ["--variant", effort]
        for image in images:
            argv += ["--file", image]
        return argv
    raise ValueError("El chat visual solo está disponible para Codex, Claude Code y OpenCode")


def turn_environment(command: Sequence[str], *, mode: str) -> dict[str, str]:
    """Extra environment variables a turn needs on top of the inherited one."""
    if provider_for(command) == "OpenCode" and mode == "read-only":
        return {"OPENCODE_PERMISSION": json.dumps(_OPENCODE_READ_ONLY_PERMISSION)}
    return {}


def normalize(provider: str, record: Any) -> list[Event]:
    if not isinstance(record, dict):
        return []
    if provider == "OpenCode":
        return normalize_opencode(record)
    return _codex(record) if provider == "Codex" else _claude(record)


def _codex(record: dict[str, Any]) -> list[Event]:
    kind = record.get("type")
    if kind == "thread.started" and isinstance(record.get("thread_id"), str):
        return [{"type": "session", "session_id": record["thread_id"]}]
    if kind == "turn.completed":
        usage = record.get("usage") if isinstance(record.get("usage"), dict) else {}
        return [_usage(usage.get("input_tokens"), usage.get("output_tokens"), None)]
    if kind in {"turn.failed", "error"}:
        error = record.get("error")
        message = error.get("message") if isinstance(error, dict) else record.get("message")
        return [{"type": "error", "message": str(message or "Codex terminó el turno con error")}]
    item = record.get("item")
    if kind not in {"item.started", "item.updated", "item.completed"} or not isinstance(item, dict):
        return []
    item_type = item.get("type")
    if item_type == "agent_message" and kind == "item.completed":
        return [{"type": "message", "role": "assistant", "text": str(item.get("text", ""))}]
    if item_type == "reasoning" and kind == "item.completed":
        return [{"type": "reasoning", "text": str(item.get("text", ""))}]
    if item_type == "error":
        return [{"type": "error", "message": str(item.get("message", ""))}]
    tool = _codex_tool(item)
    return [tool] if tool else []


def _codex_tool(item: dict[str, Any]) -> Event | None:
    item_type = item.get("type")
    status = str(item.get("status") or "")
    output = ""
    if item_type == "command_execution":
        kind, title = "command", _strip_shell(str(item.get("command", "")))
        output = str(item.get("aggregated_output") or "")
        if status == "completed" and item.get("exit_code") not in (0, None):
            status = "failed"
    elif item_type == "file_change":
        changes = item.get("changes") if isinstance(item.get("changes"), list) else []
        kind = "file_change"
        title = ", ".join(
            f"{change.get('kind', 'update')} {change.get('path', '')}" for change in changes if isinstance(change, dict)
        ) or "Cambios de archivos"
    elif item_type == "mcp_tool_call":
        kind, title = "mcp", f"{item.get('server', 'mcp')}.{item.get('tool', '')}"
    elif item_type == "web_search":
        kind, title = "search", str(item.get("query", "Búsqueda web"))
    elif item_type == "todo_list":
        entries = item.get("items") if isinstance(item.get("items"), list) else []
        kind = "todo"
        title = " · ".join(
            f"{'✓' if entry.get('completed') else '○'} {entry.get('text', '')}" for entry in entries if isinstance(entry, dict)
        ) or "Plan"
    else:
        return None
    return _tool(str(item.get("id", "")), kind, title, _status(status), output)


def _claude(record: dict[str, Any]) -> list[Event]:
    kind = record.get("type")
    if kind == "system" and record.get("subtype") == "init" and isinstance(record.get("session_id"), str):
        return [{"type": "session", "session_id": record["session_id"]}]
    if kind == "result":
        usage = record.get("usage") if isinstance(record.get("usage"), dict) else {}
        # Claude reports uncached input separately from cache reads/writes; the
        # tokens actually processed (and the context in use) are their sum.
        context = sum(
            value for key in ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens")
            if isinstance(value := usage.get(key), int)
        )
        events = [_usage(context, usage.get("output_tokens"), record.get("total_cost_usd"))]
        window = _claude_context_window(record.get("modelUsage"))
        if context or window:
            events.append({"type": "limits", "context_tokens": context or None, "context_window": window,
                           "primary": None, "secondary": None, "plan": None})
        if record.get("is_error"):
            events.append({"type": "error", "message": str(record.get("result") or record.get("subtype") or "Error")})
        return events
    message = record.get("message")
    content = message.get("content") if isinstance(message, dict) else None
    if kind not in {"assistant", "user"} or not isinstance(content, list) or record.get("isSidechain"):
        return []
    events: list[Event] = []
    for block in content:
        if not isinstance(block, dict):
            continue
        block_type = block.get("type")
        if kind == "assistant" and block_type == "text" and block.get("text"):
            events.append({"type": "message", "role": "assistant", "text": str(block["text"])})
        elif kind == "assistant" and block_type == "thinking" and block.get("thinking"):
            events.append({"type": "reasoning", "text": str(block["thinking"])})
        elif kind == "assistant" and block_type == "tool_use":
            name = str(block.get("name", "Herramienta"))
            events.append(_tool(str(block.get("id", "")), _claude_kind(name), _claude_title(name, block.get("input")), "running", ""))
        elif kind == "user" and block_type == "tool_result":
            status = "failed" if block.get("is_error") else "completed"
            events.append(_tool(str(block.get("tool_use_id", "")), "", "", status, _text_of(block.get("content"))))
    return events


def _claude_context_window(model_usage: Any) -> int | None:
    if not isinstance(model_usage, dict):
        return None
    windows = [
        entry["contextWindow"] for entry in model_usage.values()
        if isinstance(entry, dict) and isinstance(entry.get("contextWindow"), int)
    ]
    return max(windows) if windows else None


def _claude_kind(name: str) -> str:
    if name == "Bash":
        return "command"
    if name in {"Edit", "MultiEdit", "Write", "NotebookEdit"}:
        return "file_change"
    if name.startswith("mcp__"):
        return "mcp"
    if name in {"WebSearch", "WebFetch", "Grep", "Glob"}:
        return "search"
    if name == "TodoWrite":
        return "todo"
    return "tool"


def _claude_title(name: str, raw_input: Any) -> str:
    data = raw_input if isinstance(raw_input, dict) else {}
    for key in ("command", "file_path", "notebook_path", "pattern", "query", "url", "description"):
        if isinstance(data.get(key), str) and data[key]:
            return f"{name}: {data[key]}"
    return name


def _text_of(content: Any) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(str(item.get("text", "")) for item in content if isinstance(item, dict))
    return ""


def _tool(identifier: str, kind: str, title: str, status: str, output: str) -> Event:
    if len(output) > _MAX_OUTPUT_CHARS:
        output = output[:_MAX_OUTPUT_CHARS] + "\n…"
    return {"type": "tool", "id": identifier, "kind": kind, "title": title, "status": status, "output": output}


def _usage(input_tokens: Any, output_tokens: Any, cost: Any) -> Event:
    return {
        "type": "usage",
        "input_tokens": input_tokens if isinstance(input_tokens, int) else 0,
        "output_tokens": output_tokens if isinstance(output_tokens, int) else 0,
        "cost_usd": cost if isinstance(cost, (int, float)) else None,
    }


def _status(raw: str) -> str:
    if raw in {"completed", "failed"}:
        return raw
    if raw in {"declined", "error"}:
        return "failed"
    return "running"


def _strip_shell(command: str) -> str:
    for prefix in ("/bin/bash -lc ", "bash -lc ", "/bin/sh -c ", "sh -c "):
        if command.startswith(prefix):
            inner = command[len(prefix):]
            return inner[1:-1] if len(inner) >= 2 and inner[0] == inner[-1] and inner[0] in "'\"" else inner
    return command
