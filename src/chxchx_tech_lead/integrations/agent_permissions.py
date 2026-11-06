"""Turn "the agent was not allowed to do that" into a `permission` event Studio can show as a card.

Event: {"type": "permission", "id": str, "tool": str, "summary": str, "detail": str, "certain": bool,
        "choices": [choice]} where choice = {"label": str, "allow": [str], "network": bool,
        "mode": str | None, "persist": bool}. Studio applies the choice to the next turn (one-off, or
for the rest of the chat when `persist`) and asks the agent to try again.

What each CLI reports differs, so how sure we are does too:
- Claude lists `permission_denials` in the final result: certain.
- OpenCode rejects an `ask` permission in a headless run and says so in the tool error: certain.
- Codex never asks in `exec`; its sandbox just makes the command fail. Only known failure texts are
  flagged, and as "possible".
"""

from __future__ import annotations

import re
from typing import Any
from urllib.parse import urlparse

Event = dict[str, Any]

_MAX_DETAIL = 300
_SANDBOX_FAILURES = re.compile(
    r"could not resolve host|temporary failure in name resolution|network is unreachable|"
    r"read-only file system|operation not permitted|sandbox",
    re.IGNORECASE,
)
_OPENCODE_REJECTED = "rejected permission"
_OPENCODE_TOOLS = {"write": "edit", "patch": "edit", "multiedit": "edit", "apply_patch": "edit"}


def _choice(label: str, *, allow: tuple[str, ...] = (), network: bool = False, mode: str | None = None,
            persist: bool = False) -> dict[str, Any]:
    return {"label": label, "allow": list(allow), "network": network, "mode": mode, "persist": persist}


def _event(identifier: str, tool: str, summary: str, detail: str, certain: bool, choices: list[dict[str, Any]]) -> Event:
    return {"type": "permission", "id": identifier, "tool": tool, "summary": summary[:_MAX_DETAIL],
            "detail": detail[:_MAX_DETAIL], "certain": certain, "choices": choices}


def claude_denials(record: dict[str, Any]) -> list[Event]:
    denials = record.get("permission_denials")
    if not isinstance(denials, list):
        return []
    events = []
    for denial in denials:
        if not isinstance(denial, dict):
            continue
        name = str(denial.get("tool_name") or "Herramienta")
        data = denial.get("tool_input") if isinstance(denial.get("tool_input"), dict) else {}
        events.append(_claude_event(str(denial.get("tool_use_id") or name), name, data))
    return events


def _claude_event(identifier: str, name: str, data: dict[str, Any]) -> Event:
    if name == "Bash" and isinstance(data.get("command"), str):
        command = data["command"].strip()
        first = command.split()[0] if command.split() else command
        choices = []
        if "(" not in command and ")" not in command:
            choices.append(_choice("Permitir este comando", allow=(f"Bash({command})",)))
        choices.append(_choice(f"Permitir «{first}» en este chat", allow=(f"Bash({first}:*)",), persist=True))
        return _event(identifier, name, f"Ejecutar un comando: {command}", str(data.get("description") or ""), True, choices)
    if name == "WebFetch" and isinstance(data.get("url"), str):
        host = urlparse(data["url"]).hostname or ""
        rule = f"WebFetch(domain:{host})" if host else "WebFetch"
        return _event(identifier, name, f"Leer una página web: {data['url']}", "", True,
                      [_choice("Permitir esta vez", allow=(rule,)), _choice("Permitir la red en este chat", network=True, persist=True)])
    target = next((str(data[key]) for key in ("file_path", "path", "pattern", "query", "url") if data.get(key)), "")
    summary = f"{name}: {target}" if target else name
    return _event(identifier, name, summary, "", True,
                  [_choice("Permitir esta vez", allow=(name,)), _choice(f"Permitir {name} en este chat", allow=(name,), persist=True)])


def opencode_denial(part: dict[str, Any], title: str) -> list[Event]:
    state = part.get("state") if isinstance(part.get("state"), dict) else {}
    if _OPENCODE_REJECTED not in str(state.get("error") or "").casefold():
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
