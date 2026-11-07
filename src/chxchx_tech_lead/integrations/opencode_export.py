"""Read an OpenCode session through its CLI, whatever the version (1.x `export`, 2.x `session export`)."""

from __future__ import annotations

from pathlib import Path
from typing import Any


def updated_value(session: dict[str, Any]) -> Any:
    """`time.updated` in 1.x, `updated` in 2.x."""
    time_data = session.get("time")
    value = time_data.get("updated") if isinstance(time_data, dict) else None
    return value if value is not None else session.get("updated")


def export_session(run_json: Any, binary: str, session_id: str, cwd: Path) -> Any:
    """The session as `{"info": {"directory"}, "messages": [{"info": {"role"}, "parts": [{"type": "text", "text"}]}]}`."""
    for command in (["session", "export", session_id, "--sanitize"], ["export", session_id, "--sanitize"]):
        exported = run_json([binary, *command], cwd)
        if isinstance(exported, dict) and isinstance(exported.get("messages"), list):
            return _normalized(exported)
    return None


def _normalized(exported: dict[str, Any]) -> dict[str, Any]:
    """2.x writes `{"type": "user", "text"}` / `{"type": "assistant", "content": [...]}`; give them the 1.x shape."""
    info = exported.get("info") if isinstance(exported.get("info"), dict) else {}
    location = info.get("location") if isinstance(info.get("location"), dict) else {}
    if not location.get("directory") and not any(isinstance(m, dict) and m.get("type") in {"user", "assistant"}
                                                 for m in exported["messages"]):
        return exported  # 1.x already
    messages: list[dict[str, Any]] = []
    for message in exported["messages"]:
        if not isinstance(message, dict):
            continue
        kind = message.get("type")
        if kind == "user" and isinstance(message.get("text"), str):
            messages.append({"info": {"role": "user"}, "parts": [{"type": "text", "text": message["text"]}]})
        elif kind == "assistant" and isinstance(message.get("content"), list):
            parts = [{"type": "text", "text": item["text"]} for item in message["content"]
                     if isinstance(item, dict) and item.get("type") == "text" and isinstance(item.get("text"), str)]
            messages.append({"info": {"role": "assistant"}, "parts": parts})
        elif "info" in message:
            messages.append(message)
    directory = info.get("directory") or location.get("directory")
    return {"info": {**info, **({"directory": directory} if directory else {})}, "messages": messages}
