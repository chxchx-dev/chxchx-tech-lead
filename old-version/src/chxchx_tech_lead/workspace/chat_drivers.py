"""Per-provider drivers of a visual-chat turn: how the agent's output is read and how it is asked questions."""

from __future__ import annotations

import json
import subprocess
from collections.abc import Iterator
from pathlib import Path
from typing import IO, Any

from ..integrations.agent_access import Access, opencode_agent
from ..integrations.agent_codex_live import LiveCodex, configured_model
from ..integrations.agent_live import LiveClaude
from ..integrations.agent_opencode_live import LiveOpenCode
from ..integrations.agent_opencode_live2 import LiveOpenCodeV2


def json_lines(stream: IO[str]) -> Iterator[dict[str, Any]]:
    for line in stream:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            yield record


def live_driver(plan: Any, process: subprocess.Popen[str], images: list[Path]) -> LiveClaude | LiveCodex | LiveOpenCode | LiveOpenCodeV2 | None:
    if not plan.live:
        return None
    stdin = process.stdin
    assert stdin is not None
    if plan.provider == "OpenCode":
        driver = LiveOpenCodeV2 if (plan.opencode_major or 0) >= 2 else LiveOpenCode  # OpenCode 2 has another server API
        return driver(process, password=plan.env["OPENCODE_SERVER_PASSWORD"], cwd=plan.cwd, session_id=plan.session_id,
                      agent=opencode_agent(Access(plan.mode, plan.network, plan.allow)), model=plan.model, effort=plan.effort,
                      images=images)
    if plan.provider == "Codex":
        return LiveCodex(stdin, cwd=plan.cwd, access=Access(plan.mode, plan.network, plan.allow), session_id=plan.session_id,
                         model=plan.model or configured_model(plan.command), effort=plan.effort, images=images)
    return LiveClaude(stdin)
