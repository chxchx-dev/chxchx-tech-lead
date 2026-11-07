"""How a headless OpenCode turn is launched, for OpenCode 1.x and 2.x.

OpenCode 2 changed what Studio relies on: `--variant` is gone (the variant travels in the model as
`provider/model#variant`), permissions come from an ordered ruleset (`permissions`, tool `shell`) and not from
`OPENCODE_PERMISSION`, `run` talks to a shared background service unless `--standalone` is given (so that the
per-turn configuration reaches it), and the local server of 1.x (`opencode serve`, used for live approvals) has a
different API (`agent_opencode_live2`). A headless turn uses `run --standalone` on 2.x.
"""

from __future__ import annotations

import json
import re
import subprocess
from collections.abc import Sequence
from pathlib import Path

from .agent_access import Access, opencode_agent, opencode_environment
from .executable_search import with_search_path

_VERSIONS: dict[str, int | None] = {}


def opencode_major(executable: str) -> int | None:
    """Major version of the installed OpenCode (1 or 2), or None when it cannot be read."""
    key = str(Path(executable))
    if key not in _VERSIONS:
        try:
            result = subprocess.run([executable, "--version"], capture_output=True, text=True, timeout=10, check=False,
                                    env=with_search_path())
            match = re.search(r"(\d+)\.\d+", result.stdout + result.stderr)
            _VERSIONS[key] = int(match.group(1)) if match else None
        except (OSError, subprocess.TimeoutExpired):
            _VERSIONS[key] = None
    return _VERSIONS[key]


def _rule(action: str, effect: str, resource: str = "*") -> dict[str, str]:
    return {"action": action, "resource": resource, "effect": effect}


def environment(access: Access, major: int | None) -> dict[str, str]:
    if major is None or major < 2:
        return opencode_environment(access)
    rules: list[dict[str, str]] = []
    if access.mode == "read-only":
        rules += [_rule("edit", "deny"), _rule("shell", "deny")]
    elif access.mode != "full-access":
        # The last matching rule wins; `ask` has nobody to answer in a headless run, so it is rejected and becomes a card.
        rules += [_rule("shell", "allow" if access.mode == "workspace-commands" else "ask"),
                  _rule("webfetch", "allow" if access.network else "ask"),
                  _rule("websearch", "allow" if access.network else "ask")]
    for item in access.allow:
        name, _, pattern = item.partition(":")
        rules.append(_rule({"bash": "shell"}.get(name, name), "allow", pattern or "*"))
    return {"OPENCODE_CONFIG_CONTENT": json.dumps({"permissions": rules})} if rules else {}


def argv(executable: str, configured: Sequence[str], access: Access, *, session_id: str | None, model: str | None,
         effort: str | None, images: Sequence[str], major: int | None) -> list[str]:
    modern = major is not None and major >= 2
    command = [executable, "run", *(["--standalone"] if modern else []), *configured, "--format", "json",
               "--agent", opencode_agent(access)]
    if session_id:
        command += ["--session", session_id]
    if modern and model and effort and "#" not in model:
        command += ["--model", f"{model}#{effort}"]
    else:
        if model:
            command += ["--model", model]
        if effort and not modern:
            command += ["--variant", effort]
    for image in images:
        command += ["--file", image]
    return command
