from __future__ import annotations

import pytest

from chxchx_tech_lead.integrations.agent_stream import normalize, provider_for, turn_command

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
    assert provider_for(["opencode"]) is None


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
        turn_command(["opencode"], mode="read-only", session_id=None)


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
