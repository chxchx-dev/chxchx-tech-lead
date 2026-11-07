"""OpenCode 2 changed flags, permissions and its local API: a headless turn must follow."""

from __future__ import annotations

import json
import os
import stat
from pathlib import Path

import pytest

from chxchx_tech_lead.integrations import agent_opencode_turn
from chxchx_tech_lead.integrations.agent_access import Access
from chxchx_tech_lead.integrations.agent_permissions import opencode_denial
from chxchx_tech_lead.integrations.agent_stream import turn_command, turn_environment
from chxchx_tech_lead.integrations.agent_stream_opencode import normalize_opencode


def _command(major, **options):
    return turn_command(["opencode"], mode=options.pop("mode", "workspace-write"), session_id=options.pop("session_id", None),
                        opencode_major=major, **options)


def test_version_two_runs_standalone_and_puts_the_effort_in_the_model() -> None:
    argv = _command(2, model="anthropic/claude-sonnet-5-5", effort="high", session_id="ses_1", images=["/p/a.png"])

    assert argv[:3] == ["opencode", "run", "--standalone"] and "--variant" not in argv
    assert argv[argv.index("--model") + 1] == "anthropic/claude-sonnet-5-5#high"
    assert argv[argv.index("--session") + 1] == "ses_1" and argv[-2:] == ["--file", "/p/a.png"]
    assert argv[argv.index("--agent") + 1] == "build" and argv[argv.index("--format") + 1] == "json"


def test_version_one_keeps_variant_and_the_shared_service() -> None:
    argv = _command(1, model="a/b", effort="high")

    assert "--standalone" not in argv and argv[argv.index("--variant") + 1] == "high" and argv[argv.index("--model") + 1] == "a/b"
    assert _command(None, effort="low")[-2:] == ["--variant", "low"]  # unknown: the behaviour of 1.x


def test_each_access_level_becomes_an_ordered_ruleset_for_version_two() -> None:
    def rules(mode, network=False, allow=()):
        return json.loads(turn_environment(["opencode"], mode=mode, network=network, allow=allow, opencode_major=2)["OPENCODE_CONFIG_CONTENT"])["permissions"]

    assert rules("read-only") == [{"action": "edit", "resource": "*", "effect": "deny"}, {"action": "shell", "resource": "*", "effect": "deny"}]
    assert {"action": "shell", "resource": "*", "effect": "ask"} in rules("workspace-write")
    assert {"action": "shell", "resource": "*", "effect": "allow"} in rules("workspace-commands", network=True)
    assert {"action": "webfetch", "resource": "*", "effect": "allow"} in rules("workspace-commands", network=True)
    approved = rules("workspace-write", allow=("bash:git *", "webfetch"))
    assert approved[-2:] == [{"action": "shell", "resource": "git *", "effect": "allow"}, {"action": "webfetch", "resource": "*", "effect": "allow"}]
    assert approved.index({"action": "shell", "resource": "*", "effect": "ask"}) < len(approved) - 2   # specific allows come last: they win
    assert "OPENCODE_CONFIG_CONTENT" not in turn_environment(["opencode"], mode="full-access", opencode_major=2)
    assert "OPENCODE_PERMISSION" in turn_environment(["opencode"], mode="read-only", opencode_major=1)  # 1.x as before


def test_a_rejected_request_of_version_two_becomes_a_permission_card() -> None:
    record = {"type": "tool_use", "sessionID": "s", "part": {"type": "tool", "callID": "c9", "tool": "shell", "state": {
        "status": "error", "input": {"command": "echo hola"},
        "error": "This non-interactive run cannot ask the user for permission, so the request was rejected. Continue without this action."}}}

    events = normalize_opencode(record)

    assert [e["type"] for e in events] == ["tool", "permission"] and events[0]["kind"] == "command"
    card = events[1]
    assert card["tool"] == "shell" and card["choices"][0]["allow"] == ["bash"]      # Studio re-runs with --allow bash
    assert not opencode_denial({"tool": "shell", "state": {"status": "error", "error": "no such file"}}, "shell: ls")


def _fake_opencode(directory: Path, version_output: str) -> str:
    directory.mkdir(parents=True, exist_ok=True)
    program = directory / "opencode"
    program.write_text(f"#!/bin/sh\necho '{version_output}'\n", encoding="utf-8")
    program.chmod(program.stat().st_mode | stat.S_IXUSR)
    return str(program)


@pytest.mark.skipif(os.name == "nt", reason="uses a shell script")
def test_the_installed_version_is_read_once_per_executable(tmp_path: Path) -> None:
    agent_opencode_turn._VERSIONS.clear()
    two = _fake_opencode(tmp_path / "a", "v2.2.6")
    one = _fake_opencode(tmp_path / "b", "1.19.5")

    assert agent_opencode_turn.opencode_major(two) == 2 and agent_opencode_turn.opencode_major(one) == 1
    assert agent_opencode_turn.opencode_major(str(tmp_path / "no-existe")) is None
    assert agent_opencode_turn.supports_live(1) and agent_opencode_turn.supports_live(None) and not agent_opencode_turn.supports_live(2)
    assert agent_opencode_turn.environment(Access("workspace-write", False, ()), 2)["OPENCODE_CONFIG_CONTENT"]
