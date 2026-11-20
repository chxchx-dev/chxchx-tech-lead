"""Turn "the agent was not allowed to do that" into a `permission` event Studio can show as a card.

Event: {"type": "permission", "id": str, "tool": str, "summary": str, "detail": str, "certain": bool,
        "choices": [choice]} where choice = {"label": str, "allow": [str], "network": bool,
        "mode": str | None, "persist": bool}. Studio applies the choice to the next turn (one-off, or
for the rest of the chat when `persist`) and asks the agent to try again.

What each CLI reports differs, so how sure we are does too:
- OpenCode rejects an `ask` permission in a headless run and says so in the tool error: certain.
- Codex never asks in `exec`; its sandbox just makes the command fail. Only known failure texts are
  flagged, and as "possible".
"""

from __future__ import annotations

import re
from typing import Any

Event = dict[str, Any]

_MAX_DETAIL = 300
_SANDBOX_FAILURES = re.compile(
    r"could not resolve host|temporary failure in name resolution|network is unreachable|"
    r"read-only file system|operation not permitted|sandbox",
    re.IGNORECASE,
)
# What a tool error says when OpenCode 1.x / 2.x rejected the request because nobody could answer it.
_OPENCODE_REJECTED = ("rejected permission", "cannot ask the user for permission")
_OPENCODE_TOOLS = {"write": "edit", "patch": "edit", "multiedit": "edit", "apply_patch": "edit", "shell": "bash"}


def _choice(label: str, *, allow: tuple[str, ...] = (), network: bool = False, mode: str | None = None,
            persist: bool = False) -> dict[str, Any]:
    return {"label": label, "allow": list(allow), "network": network, "mode": mode, "persist": persist}


def _event(identifier: str, tool: str, summary: str, detail: str, certain: bool, choices: list[dict[str, Any]]) -> Event:
    return {"type": "permission", "id": identifier, "tool": tool, "summary": summary[:_MAX_DETAIL],
            "detail": detail[:_MAX_DETAIL], "certain": certain, "choices": choices}


def opencode_denial(part: dict[str, Any], title: str) -> list[Event]:
    state = part.get("state") if isinstance(part.get("state"), dict) else {}
    error = str(state.get("error") or state.get("output") or "").casefold()
    if not any(marker in error for marker in _OPENCODE_REJECTED):
        return []
    name = str(part.get("tool") or "tool")
    permission = _OPENCODE_TOOLS.get(name.casefold(), name.casefold())
    label = {"bash": "comandos", "edit": "ediciones", "webfetch": "la web"}.get(permission, name)
    identifier = str(part.get("callID") or part.get("id") or name)
    return [_event(identifier, name, title, "", True, [
        _choice("Permitir esta vez", allow=(permission,)),
        _choice(f"Permitir {label} en este chat", allow=(permission,), persist=True),
    ])]


def codex_block(item: dict[str, Any]) -> list[Event]:
    if item.get("type") != "command_execution":
        return []
    output = str(item.get("aggregated_output") or "")
    if not _SANDBOX_FAILURES.search(output):
        return []
    command = str(item.get("command") or "")
    return [_event(str(item.get("id") or command), "command", f"Un comando pudo bloquearlo el sandbox: {command}", output.strip()[-200:],
                   False, [_choice("Permitir la red en este chat", network=True, persist=True),
                           _choice("Acceso total solo para reintentar", mode="full-access")])]
