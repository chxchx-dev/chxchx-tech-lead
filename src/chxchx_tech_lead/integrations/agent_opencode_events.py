"""Small translations of OpenCode 2.x events into Studio events."""

from __future__ import annotations

from typing import Any

Event = dict[str, Any]


def text_of(content: Any) -> str:
    if not isinstance(content, list):
        return ""
    return "\n".join(str(item.get("text")) for item in content if isinstance(item, dict) and item.get("type") == "text" and item.get("text"))


def usage_events(data: dict[str, Any]) -> list[Event]:
    tokens = data.get("tokens") if isinstance(data.get("tokens"), dict) else {}
    cache = tokens.get("cache") if isinstance(tokens.get("cache"), dict) else {}
    count = lambda value: value if isinstance(value, int) and value > 0 else 0  # noqa: E731
    context = count(tokens.get("input")) + count(cache.get("read")) + count(cache.get("write"))  # cached tokens are input too
    cost = data.get("cost")
    return [{"type": "usage", "input_tokens": context, "output_tokens": count(tokens.get("output")),
             "cost_usd": cost if isinstance(cost, (int, float)) else None},
            {"type": "limits", "context_tokens": context or None, "context_window": None, "primary": None, "secondary": None, "plan": None}]
