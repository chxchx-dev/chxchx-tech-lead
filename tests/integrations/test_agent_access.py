"""Security levels per CLI, and the `permission` events built from real denials."""

from __future__ import annotations

import json

import pytest

from chxchx_tech_lead.integrations.agent_access import LEVELS
from chxchx_tech_lead.integrations.agent_stream import normalize, turn_command, turn_environment


def codex(mode, **kw):
    return turn_command(["codex"], mode=mode, session_id=None, **kw)


def claude(mode, **kw):
    return turn_command(["claude"], mode=mode, session_id=None, **kw)


def test_every_level_is_known_and_unknown_ones_are_refused() -> None:
    assert LEVELS == ("read-only", "workspace-write", "workspace-commands", "full-access")
    for mode in LEVELS:
        codex(mode), claude(mode), turn_command(["opencode"], mode=mode, session_id=None)
    with pytest.raises(ValueError):
        codex("yolo")


def test_codex_maps_levels_to_its_sandbox() -> None:
    assert codex("read-only")[codex("read-only").index("-s") + 1] == "read-only"
    assert codex("workspace-write")[codex("workspace-write").index("-s") + 1] == "workspace-write"
    assert codex("workspace-commands")[codex("workspace-commands").index("-s") + 1] == "workspace-write"
    assert codex("full-access")[codex("full-access").index("-s") + 1] == "danger-full-access"


def test_codex_network_is_only_opened_when_the_sandbox_is_the_project() -> None:
    assert "sandbox_workspace_write.network_access=true" in codex("workspace-write", network=True)
    assert "sandbox_workspace_write.network_access=true" not in codex("workspace-write")
    assert "sandbox_workspace_write.network_access=true" not in codex("read-only", network=True)


def test_claude_levels_and_approved_rules() -> None:
    assert claude("read-only")[claude("read-only").index("--permission-mode") + 1] == "plan"
    assert "--allowedTools" not in claude("workspace-write")
    assert claude("workspace-commands")[-2:] == ["--allowedTools", "Bash"]
    assert claude("full-access")[claude("full-access").index("--permission-mode") + 1] == "bypassPermissions"
    command = claude("workspace-write", allow=["Bash(git status)", "Bash(git status)"], network=True)
    assert command[command.index("--allowedTools") + 1:] == ["Bash(git status)", "WebFetch", "WebSearch"]
    # A read-only chat never gets rules, whatever was approved before.
    assert "--allowedTools" not in claude("read-only", allow=["Bash"], network=True)


def test_opencode_permissions_per_level() -> None:
    def env(mode, **kw):
        return json.loads(turn_environment(["opencode"], mode=mode, **kw)["OPENCODE_PERMISSION"])

    assert env("read-only") == {"edit": "deny", "bash": "deny"}
    assert env("workspace-write") == {"bash": "ask", "webfetch": "ask"}
    assert env("workspace-commands", network=True) == {"bash": "allow", "webfetch": "allow"}
    assert env("workspace-write", allow=["bash"])["bash"] == "allow"
    # A rule with a pattern only opens what matches it; the last matching rule wins, so the default goes first.
    assert env("workspace-write", allow=["bash:git *", "bash:ls *"])["bash"] == {"*": "ask", "git *": "allow", "ls *": "allow"}
    assert env("workspace-commands", allow=["bash:git *"])["bash"] == {"*": "allow", "git *": "allow"}
    assert env("full-access")["external_directory"] == "allow"
    assert turn_command(["opencode"], mode="full-access", session_id=None)[5] == "build"


# Shapes captured from the real CLIs (claude 2.1.319, opencode 1.19.5, codex-cli 0.166.0).
def test_claude_denials_become_cards_with_scoped_rules() -> None:
    events = normalize("Claude", {"type": "result", "subtype": "success", "is_error": False, "usage": {}, "permission_denials": [
        {"tool_name": "Bash", "tool_use_id": "toolu_1", "tool_input": {"command": "python3 -V", "description": "Run"}}]})
    card = next(event for event in events if event["type"] == "permission")
    assert card["certain"] and card["tool"] == "Bash" and "python3" in card["summary"]
    allow = [choice["allow"] for choice in card["choices"]]
    assert ["Bash(python3 -V)"] in allow and ["Bash(python3:*)"] in allow
    assert [choice["persist"] for choice in card["choices"]] == [False, True]


def test_claude_command_with_parentheses_only_offers_the_prefix_rule() -> None:
    events = normalize("Claude", {"type": "result", "usage": {}, "permission_denials": [
        {"tool_name": "Bash", "tool_use_id": "t", "tool_input": {"command": "echo $(whoami)"}}]})
    card = next(event for event in events if event["type"] == "permission")
    assert [choice["allow"] for choice in card["choices"]] == [["Bash(echo:*)"]]


def test_claude_web_and_other_tools() -> None:
    events = normalize("Claude", {"type": "result", "usage": {}, "permission_denials": [
        {"tool_name": "WebFetch", "tool_use_id": "w", "tool_input": {"url": "https://example.com/a"}},
        {"tool_name": "Edit", "tool_use_id": "e", "tool_input": {"file_path": "/etc/hosts"}}]})
    cards = [event for event in events if event["type"] == "permission"]
    assert cards[0]["choices"][0]["allow"] == ["WebFetch(domain:example.com)"] and cards[0]["choices"][1]["network"]
    assert cards[1]["summary"] == "Edit: /etc/hosts" and cards[1]["choices"][0]["allow"] == ["Edit"]


def test_a_result_without_denials_makes_no_card() -> None:
    assert not [e for e in normalize("Claude", {"type": "result", "usage": {}, "permission_denials": []}) if e["type"] == "permission"]


def test_opencode_rejection_becomes_a_card() -> None:
    record = {"type": "tool_use", "part": {"tool": "bash", "callID": "c1", "state": {
        "status": "error", "error": "The user rejected permission to use this specific tool call.",
        "input": {"command": "echo hola > ob.txt"}}}}
    events = normalize("OpenCode", record)
    assert [event["type"] for event in events] == ["tool", "permission"]
    assert events[1]["choices"][0]["allow"] == ["bash"] and events[1]["certain"]
    ordinary = {"type": "tool_use", "part": {"tool": "bash", "state": {"status": "error", "error": "exit 1"}}}
    assert [event["type"] for event in normalize("OpenCode", ordinary)] == ["tool"]


def test_codex_flags_sandbox_failures_as_possible_blocks() -> None:
    item = {"id": "i", "type": "command_execution", "command": "curl x", "status": "completed", "exit_code": 0,
            "aggregated_output": "curl: (6) Could not resolve host: example.com\nfin\n"}
    events = normalize("Codex", {"type": "item.completed", "item": item})
    card = next(event for event in events if event["type"] == "permission")
    assert not card["certain"]
    assert {"network": True, "mode": None} == {key: card["choices"][0][key] for key in ("network", "mode")}
    assert card["choices"][1]["mode"] == "full-access" and not card["choices"][1]["persist"]
    fine = dict(item, aggregated_output="all good\n")
    assert [e["type"] for e in normalize("Codex", {"type": "item.completed", "item": fine})] == ["tool"]
    # A command that is still running is never judged.
    assert not [e for e in normalize("Codex", {"type": "item.started", "item": dict(item, status="in_progress")}) if e["type"] == "permission"]
