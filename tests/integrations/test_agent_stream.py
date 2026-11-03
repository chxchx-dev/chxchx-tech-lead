from __future__ import annotations

import pytest

from chxchx_tech_lead.integrations.agent_stream import normalize, provider_for, turn_command, turn_environment

# Captured from `codex exec --json` (codex-cli 0.166.0).
CODEX_EVENTS = [
    {"type": "thread.started", "thread_id": "01a19a6b"},
    {"type": "turn.started"},
    {"type": "item.completed", "item": {"id": "item_0", "type": "agent_message", "text": "Voy a listar."}},
    {"type": "item.started", "item": {"id": "item_1", "type": "command_execution", "command": "/bin/bash -lc ls",
                                      "aggregated_output": "", "exit_code": None, "status": "in_progress"}},
    {"type": "item.completed", "item": {"id": "item_1", "type": "command_execution", "command": "/bin/bash -lc 'tail -c 9 x'",
                                        "aggregated_output": "a\n", "exit_code": 2, "status": "completed"}},
    {"type": "item.completed", "item": {"id": "item_2", "type": "file_change", "status": "completed",
                                        "changes": [{"path": "app.py", "kind": "update"}]}},
    {"type": "turn.completed", "usage": {"input_tokens": 27116, "output_tokens": 97}},
]


def test_detects_providers_including_windows_shims() -> None:
    assert provider_for(["codex"]) == "Codex"
    assert provider_for(["C:\\npm\\claude.cmd"]) == "Claude"
    assert provider_for(["opencode"]) == "OpenCode"
    assert provider_for(["ollama", "launch", "opencode"]) is None


def test_codex_turn_reads_prompt_from_stdin_and_resumes() -> None:
    assert turn_command(["codex", "-m", "gpt-5"], mode="read-only", session_id=None) == [
        "codex", "exec", "-m", "gpt-5", "--json", "-s", "read-only", "--skip-git-repo-check", "-"
    ]
    assert turn_command(["codex"], mode="workspace-write", session_id="t1")[-4:] == [
        "--skip-git-repo-check", "resume", "t1", "-"
    ]


def test_claude_turn_uses_stream_json_and_permission_mode() -> None:
    command = turn_command(["claude"], mode="workspace-write", session_id="s1")
    assert command[:5] == ["claude", "-p", "--output-format", "stream-json", "--verbose"]
    assert command[-4:] == ["--permission-mode", "acceptEdits", "--resume", "s1"]
    assert "plan" in turn_command(["claude"], mode="read-only", session_id=None)


def test_rejects_unknown_mode_and_provider() -> None:
    with pytest.raises(ValueError):
        turn_command(["codex"], mode="yolo", session_id=None)
    with pytest.raises(ValueError):
        turn_command(["aider"], mode="read-only", session_id=None)


def test_normalizes_codex_events() -> None:
    events = [event for record in CODEX_EVENTS for event in normalize("Codex", record)]

    assert events[0] == {"type": "session", "session_id": "01a19a6b"}
    assert events[1] == {"type": "message", "role": "assistant", "text": "Voy a listar."}
    assert events[2]["title"] == "ls" and events[2]["status"] == "running"
    assert events[3]["title"] == "tail -c 9 x" and events[3]["status"] == "failed" and events[3]["output"] == "a\n"
    assert events[4]["kind"] == "file_change" and events[4]["title"] == "update app.py"
    assert events[5] == {"type": "usage", "input_tokens": 27116, "output_tokens": 97, "cost_usd": None}


def test_normalizes_claude_stream_json() -> None:
    records = [
        {"type": "system", "subtype": "init", "session_id": "s-claude"},
        {"type": "assistant", "message": {"content": [
            {"type": "text", "text": "Reviso el archivo."},
            {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {"command": "cat app.py"}},
        ]}},
        {"type": "user", "message": {"content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": [{"type": "text", "text": "print('hola')"}]},
        ]}},
        {"type": "assistant", "isSidechain": True, "message": {"content": [{"type": "text", "text": "subagente"}]}},
        {"type": "result", "subtype": "success", "is_error": False, "total_cost_usd": 0.01,
         "usage": {"input_tokens": 10, "output_tokens": 5}},
    ]
    events = [event for record in records for event in normalize("Claude", record)]

    assert [event["type"] for event in events] == ["session", "message", "tool", "tool", "usage", "limits"]
    assert events[2]["title"] == "Bash: cat app.py" and events[2]["kind"] == "command"
    assert events[3]["id"] == "toolu_1" and events[3]["status"] == "completed" and events[3]["output"] == "print('hola')"
    assert events[4]["cost_usd"] == 0.01


def test_codex_flags_for_model_effort_and_images_end_before_the_prompt() -> None:
    command = turn_command(["codex"], mode="read-only", session_id="t1", model="gpt-x", effort="high",
                           images=["a.png", "b.png"])
    assert command[command.index("-m") + 1] == "gpt-x"
    assert 'model_reasoning_effort="high"' in command
    assert command[-6:] == ["--image", "b.png", "--skip-git-repo-check", "resume", "t1", "-"]


def test_claude_flags_for_model_and_effort_and_context_limits() -> None:
    command = turn_command(["claude"], mode="read-only", session_id=None, model="opus", effort="max")
    assert command[-4:] == ["--model", "opus", "--effort", "max"]

    events = normalize("Claude", {"type": "result", "is_error": False, "total_cost_usd": 0.2,
                                  "usage": {"input_tokens": 10, "cache_read_input_tokens": 90, "output_tokens": 5},
                                  "modelUsage": {"claude-opus": {"contextWindow": 200000}}})
    assert events[1] == {"type": "limits", "context_tokens": 100, "context_window": 200000,
                         "primary": None, "secondary": None, "plan": None}
    assert events[0]["input_tokens"] == 100 and events[0]["cost_usd"] == 0.2


# Shape documented by community parsers of `opencode run --format json`; the CLI
# was not available to capture it, so the fields are kept to what they agree on.
OPENCODE_EVENTS = [
    {"type": "step_start", "sessionID": "ses_abc", "part": {"type": "step-start"}},
    {"type": "reasoning", "sessionID": "ses_abc", "part": {"type": "reasoning", "text": "Pienso."}},
    {"type": "tool_use", "sessionID": "ses_abc", "part": {"type": "tool", "callID": "c1", "tool": "bash", "state": {
        "status": "completed", "input": {"command": "ls"}, "output": "a.py\n", "title": "ls"}}},
    {"type": "tool_use", "sessionID": "ses_abc", "part": {"type": "tool", "callID": "c2", "tool": "edit", "state": {
        "status": "error", "input": {"filePath": "app.py"}, "error": "no coincide"}}},
    {"type": "tool_use", "sessionID": "ses_abc", "part": {"type": "tool", "callID": "c3", "tool": "memory_search",
                                                          "state": {"status": "completed", "input": {"query": "x"}}}},
    {"type": "text", "sessionID": "ses_abc", "part": {"type": "text", "text": "Listo."}},
    {"type": "step_finish", "sessionID": "ses_abc", "part": {"type": "step-finish", "reason": "stop", "cost": 0.01,
        "tokens": {"input": 100, "output": 7, "reasoning": 3, "cache": {"read": 50, "write": 10}}}},
    {"type": "error", "sessionID": "ses_abc", "error": {"name": "APIError", "data": {"message": "sin cuota"}}},
]


def test_opencode_turn_selects_agent_by_mode_and_resumes_by_session() -> None:
    assert turn_command(["opencode"], mode="read-only", session_id=None) == [
        "opencode", "run", "--format", "json", "--agent", "plan"
    ]
    command = turn_command(["opencode"], mode="workspace-write", session_id="ses_1", model="anthropic/claude-sonnet-5-5",
                           effort="high", images=["/tmp/a.png"])
    assert command == ["opencode", "run", "--format", "json", "--agent", "build", "--session", "ses_1",
                       "--model", "anthropic/claude-sonnet-5-5", "--variant", "high", "--file", "/tmp/a.png"]
    assert "--auto" not in command


def test_opencode_read_only_denies_edit_and_bash_through_the_environment() -> None:
    assert turn_environment(["opencode"], mode="read-only") == {"OPENCODE_PERMISSION": '{"edit": "deny", "bash": "deny"}'}
    assert turn_environment(["opencode"], mode="workspace-write") == {}
    assert turn_environment(["codex"], mode="read-only") == {}


def test_normalizes_opencode_events() -> None:
    events = [event for record in OPENCODE_EVENTS for event in normalize("OpenCode", record)]

    assert events[0] == {"type": "session", "session_id": "ses_abc"}
    assert events[1] == {"type": "reasoning", "text": "Pienso."}
    assert events[2] == {"type": "tool", "id": "c1", "kind": "command", "title": "bash: ls",
                         "status": "completed", "output": "a.py\n"}
    assert (events[3]["kind"], events[3]["status"], events[3]["output"]) == ("file_change", "failed", "no coincide")
    assert events[4]["kind"] == "mcp"
    assert events[5] == {"type": "message", "role": "assistant", "text": "Listo."}
    assert events[6] == {"type": "usage", "input_tokens": 160, "output_tokens": 7, "cost_usd": 0.01}
    assert events[7]["context_tokens"] == 160 and events[7]["context_window"] is None
    assert events[8] == {"type": "error", "message": "sin cuota"}


def test_opencode_ignores_unknown_or_malformed_records() -> None:
    assert normalize("OpenCode", {"type": "mystery"}) == []
    assert normalize("OpenCode", {"type": "text", "part": "no es un objeto"}) == []
    assert normalize("OpenCode", {"type": "step_finish"})[0]["input_tokens"] == 0


def test_ephemeral_turns_ask_each_cli_not_to_keep_the_session() -> None:
    assert "--ephemeral" in turn_command(["codex"], mode="read-only", session_id=None, ephemeral=True)
    assert "--no-session-persistence" in turn_command(["claude"], mode="read-only", session_id=None, ephemeral=True)
    # OpenCode has no such flag (its session is deleted afterwards), and normal turns keep their sessions.
    assert turn_command(["opencode"], mode="read-only", session_id=None, ephemeral=True) == turn_command(["opencode"], mode="read-only", session_id=None)
    assert "--ephemeral" not in turn_command(["codex"], mode="read-only", session_id=None)
    assert "--no-session-persistence" not in turn_command(["claude"], mode="read-only", session_id=None)
