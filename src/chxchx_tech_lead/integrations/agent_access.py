"""Security levels of a visual-chat turn and how each CLI enforces them.

The chat runs one non-interactive process per message, so nobody can answer an approval
prompt in the middle of a turn. Instead the level decides up front what the agent may do;
whatever it is not allowed to do comes back as a `permission` event (see `agent_permissions`),
and the person can grant it for the next turn.

- read-only:          looks, never changes anything.
- workspace-write:    edits the project; commands and the network need approval.
- workspace-commands: edits the project and runs commands without asking.
- full-access:        no restrictions at all (the CLI's own sandbox is switched off).
"""

from __future__ import annotations

import json
from dataclasses import dataclass

LEVELS = ("read-only", "workspace-write", "workspace-commands", "full-access")

_CODEX_SANDBOX = {
    "read-only": "read-only",
    "workspace-write": "workspace-write",
    "workspace-commands": "workspace-write",  # Codex runs commands inside its sandbox either way
    "full-access": "danger-full-access",
}
_CLAUDE_PERMISSION = {
    "read-only": "plan",
    "workspace-write": "acceptEdits",
    "workspace-commands": "acceptEdits",
    "full-access": "bypassPermissions",
}
# The built-in `plan` agent still allows bash, so read-only also denies it explicitly.
_OPENCODE_READ_ONLY = {"edit": "deny", "bash": "deny"}
_OPENCODE_FULL = {"edit": "allow", "bash": "allow", "webfetch": "allow", "external_directory": "allow"}


@dataclass(frozen=True, slots=True)
class Access:
    mode: str = "read-only"
    network: bool = False
    # Provider-specific rules the person already approved (Claude tool rules, OpenCode permission names).
    allow: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if self.mode not in LEVELS:
            raise ValueError(f"Modo no soportado: {self.mode}")

    @property
    def edits(self) -> bool:
        return self.mode != "read-only"


def codex_arguments(access: Access) -> list[str]:
    argv = ["-s", _CODEX_SANDBOX[access.mode]]
    if access.network and access.mode in {"workspace-write", "workspace-commands"}:
        argv += ["-c", "sandbox_workspace_write.network_access=true"]
    return argv


def claude_arguments(access: Access) -> list[str]:
    argv = ["--permission-mode", _CLAUDE_PERMISSION[access.mode]]
    if access.mode in {"workspace-write", "workspace-commands"}:
        rules = list(access.allow)
        if access.mode == "workspace-commands":
            rules.append("Bash")
        if access.network:
            rules += ["WebFetch", "WebSearch"]
        if rules:
            argv += ["--allowedTools", *dict.fromkeys(rules)]
    return argv


def opencode_agent(access: Access) -> str:
    return "build" if access.edits else "plan"


def opencode_environment(access: Access) -> dict[str, str]:
    if access.mode == "read-only":
        return {"OPENCODE_PERMISSION": json.dumps(_OPENCODE_READ_ONLY)}
    if access.mode == "full-access":
        return {"OPENCODE_PERMISSION": json.dumps(_OPENCODE_FULL)}
    # `ask` has nobody to answer in a headless run: OpenCode rejects it and says so, which becomes a card.
    permission = {
        "bash": "allow" if access.mode == "workspace-commands" else "ask",
        "webfetch": "allow" if access.network else "ask",
    }
    for name in access.allow:
        permission[name] = "allow"
    return {"OPENCODE_PERMISSION": json.dumps(permission)}
