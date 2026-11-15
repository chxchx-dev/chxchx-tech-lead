"""Normalize `opencode run --format json` events into `chxchx.agent-event` v1.

OpenCode prints one JSON object per line: `{"type", "timestamp", "sessionID",
"part"}`. The CLI emits `tool_use` only once a tool finishes, and every model
step ends with a `step_finish` carrying its tokens and cost. Studio sums the
`usage` events, so each step reports its own tokens.
"""

from __future__ import annotations

from typing import Any

from .agent_permissions import opencode_denial

Event = dict[str, Any]

_MAX_OUTPUT_CHARS = 4000
_FILE_TOOLS = {"edit", "write", "patch", "multiedit", "apply_patch"}
_SEARCH_TOOLS = {"grep", "glob", "list", "webfetch", "websearch", "codesearch"}
_TITLE_KEYS = ("command", "filePath", "path", "pattern", "query", "url", "description")


def normalize_opencode(record: dict[str, Any]) -> list[Event]:
    kind = record.get("type")
    part = record.get("part") if isinstance(record.get("part"), dict) else {}
    if kind == "step_start" and isinstance(record.get("sessionID"), str):
        return [{"type": "session", "session_id": record["sessionID"]}]
    if kind == "text" and part.get("text"):
        return [{"type": "message", "role": "assistant", "text": str(part["text"])}]
    if kind == "reasoning" and part.get("text"):
        return [{"type": "reasoning", "text": str(part["text"])}]
    if kind == "tool_use":
        tool = _tool(part)
        return [tool, *opencode_denial(part, tool["title"])]
    if kind == "step_finish":
        return _step_finish(part)
    if kind == "error":
        return [{"type": "error", "message": _error_message(record.get("error"))}]
    return []


def _step_finish(part: dict[str, Any]) -> list[Event]:
    tokens = part.get("tokens") if isinstance(part.get("tokens"), dict) else {}
    cache = tokens.get("cache") if isinstance(tokens.get("cache"), dict) else {}
    # Like Claude, count cached tokens as input: that is what the step processed.
    context = sum(value for value in (_count(tokens.get("input")), _count(cache.get("read")), _count(cache.get("write"))))
    cost = part.get("cost")
    return [
        {"type": "usage", "input_tokens": context, "output_tokens": _count(tokens.get("output")),
         "cost_usd": cost if isinstance(cost, (int, float)) else None},
        {"type": "limits", "context_tokens": context or None, "context_window": None,
         "primary": None, "secondary": None, "plan": None},
    ]


def _tool(part: dict[str, Any]) -> Event:
    name = str(part.get("tool") or "tool")
    state = part.get("state") if isinstance(part.get("state"), dict) else {}
    status = str(state.get("status") or "")
    failed = status == "error"
    output = str(state.get("error") if failed else state.get("output") or "")
    if len(output) > _MAX_OUTPUT_CHARS:
        output = output[:_MAX_OUTPUT_CHARS] + "\n…"
    return {
        "type": "tool",
        "id": str(part.get("callID") or part.get("id") or ""),
        "kind": _kind(name),
        "title": _title(name, state),
        "status": "failed" if failed else "completed" if status == "completed" else "running",
        "output": output,
    }


def _kind(name: str) -> str:
    lowered = name.casefold()
    if lowered in {"bash", "shell"}:
        return "command"
    if lowered in _FILE_TOOLS:
        return "file_change"
    if lowered in _SEARCH_TOOLS:
        return "search"
    if lowered in {"todowrite", "todoread"}:
        return "todo"
    # MCP tools are exposed as `<server>_<tool>`; built-ins have no underscore.
    return "mcp" if "_" in lowered else "tool"


def _title(name: str, state: dict[str, Any]) -> str:
    data = state.get("input") if isinstance(state.get("input"), dict) else {}
    if name.casefold() == "todowrite" and isinstance(data.get("todos"), list):
        entries = [item for item in data["todos"] if isinstance(item, dict)]
        if entries:
            return " · ".join(f"{'✓' if item.get('status') == 'completed' else '○'} {item.get('content', '')}" for item in entries)
    for key in _TITLE_KEYS:
        if isinstance(data.get(key), str) and data[key]:
            return f"{name}: {data[key]}"
    return str(state.get("title") or name)


def _error_message(error: Any) -> str:
    if isinstance(error, dict):
        data = error.get("data") if isinstance(error.get("data"), dict) else {}
        message = data.get("message") or error.get("message") or error.get("name")
        if message:
            return str(message)
    return str(error) if isinstance(error, str) and error else "OpenCode terminó el turno con error"


def _count(value: Any) -> int:
    return value if isinstance(value, int) and not isinstance(value, bool) else 0
