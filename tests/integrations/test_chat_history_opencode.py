from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.integrations import chat_history_opencode
from chxchx_tech_lead.integrations.chat_history_opencode import latest_opencode_conversation


def _export(*messages: tuple[str, str]) -> dict:
    return {"messages": [
        {"info": {"role": role}, "parts": [{"type": "text", "text": text}, {"type": "tool", "state": {}}]}
        for role, text in messages
    ]}


def test_reads_only_the_newest_session_that_belongs_to_the_project(tmp_path: Path, monkeypatch) -> None:
    other = tmp_path / "otro"
    other.mkdir()
    calls: list[list[str]] = []

    def fake_run_json(command, cwd):
        calls.append(command)
        if command[1:3] == ["session", "export"]:
            return None  # OpenCode 1.x has no such command
        if command[1] == "session":
            return [
                {"id": "ses_other", "directory": str(other), "time": {"updated": 1}},
                {"id": "ses_mine", "directory": str(tmp_path), "time": {"updated": 1_800_000_000_000}},
            ]
        return _export(("user", "Arregla la memoria"), ("assistant", "Listo"))

    monkeypatch.setattr(chat_history_opencode, "run_json", fake_run_json)

    conversation = latest_opencode_conversation(tmp_path, executable="opencode")

    assert conversation is not None and conversation.provider == "OpenCode"
    assert conversation.session_id == "ses_mine" and conversation.title == "Arregla la memoria"
    assert [(m.role, m.text) for m in conversation.messages] == [("user", "Arregla la memoria"), ("assistant", "Listo")]
    assert calls[0][1:3] == ["session", "list"] and calls[-1][1] == "export" and "ses_mine" in calls[-1]


def test_returns_none_without_the_cli_or_a_matching_session(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(chat_history_opencode, "find_executable", lambda name: None)
    assert latest_opencode_conversation(tmp_path) is None
    monkeypatch.setattr(chat_history_opencode, "run_json", lambda command, cwd: [])
    assert latest_opencode_conversation(tmp_path, executable="opencode") is None


def test_reads_a_version_two_session(tmp_path: Path, monkeypatch) -> None:
    """2.x: `session list` is flat (`updated`) and `session export` writes typed messages."""
    exported = {"info": {"id": "ses_two", "location": {"directory": str(tmp_path)}}, "messages": [
        {"type": "agent-switched", "agent": "build"},
        {"type": "user", "text": "Arregla la memoria"},
        {"type": "assistant", "agent": "build", "content": [{"type": "tool", "name": "shell"}, {"type": "text", "text": "Listo"}]},
        {"type": "idle"}]}

    def fake_run_json(command, cwd):
        if command[1:3] == ["session", "list"]:
            return [{"id": "ses_two", "directory": str(tmp_path), "updated": 1_800_000_000_000, "created": 1}]
        return exported if command[1:3] == ["session", "export"] else None

    monkeypatch.setattr(chat_history_opencode, "run_json", fake_run_json)

    conversation = latest_opencode_conversation(tmp_path, executable="opencode")

    assert conversation is not None and conversation.session_id == "ses_two"
    assert [(m.role, m.text) for m in conversation.messages] == [("user", "Arregla la memoria"), ("assistant", "Listo")]
    assert conversation.modified_at.year == 2027
