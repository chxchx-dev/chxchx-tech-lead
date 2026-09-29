"""Collect Claude Code's documented status-line metrics for the usage pane."""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path


def _percent(value: object) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return min(100.0, max(0.0, number))


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeError):
        return 0
    if not isinstance(payload, dict):
        return 0

    context = payload.get("context_window")
    context = context if isinstance(context, dict) else {}
    limits = payload.get("rate_limits")
    limits = limits if isinstance(limits, dict) else {}
    snapshot = {
        "context_window": {
            "used_percentage": _percent(context.get("used_percentage")),
            "remaining_percentage": _percent(context.get("remaining_percentage")),
            "total_input_tokens": context.get("total_input_tokens"),
            "context_window_size": context.get("context_window_size"),
        },
        "rate_limits": {},
        "updated_at": time.time(),
    }
    for key in ("five_hour", "seven_day"):
        window = limits.get(key)
        if isinstance(window, dict):
            snapshot["rate_limits"][key] = {
                "used_percentage": _percent(window.get("used_percentage")),
                "resets_at": window.get("resets_at"),
            }

    target = os.environ.get("CHXCHX_USAGE_SNAPSHOT")
    if target:
        path = Path(target)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = path.with_suffix(".tmp")
            temporary.write_text(json.dumps(snapshot), encoding="utf-8")
            temporary.replace(path)
        except OSError:
            pass

    pieces = []
    remaining = snapshot["context_window"]["remaining_percentage"]
    if remaining is not None:
        pieces.append(f"contexto {remaining:.0f}% libre")
    for key, label in (("five_hour", "5h"), ("seven_day", "7d")):
        window = snapshot["rate_limits"].get(key, {})
        consumed = window.get("used_percentage")
        if consumed is not None:
            pieces.append(f"{label} {max(0, 100 - round(consumed))}% libre")
    if pieces:
        print(" · ".join(pieces))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
