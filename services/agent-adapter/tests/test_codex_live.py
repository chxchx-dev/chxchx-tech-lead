"""Codex's live approvals and questions end to end, against a real fake `codex app-server` process."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest
from conftest import allow, codex_request, deny, texts, turn

from chxchx_agent_adapter.codex_live import LiveCodex


def _card(events):
    return next(e for e in events if e["type"] == "permission")


def test_a_plain_turn_streams_session_message_usage_and_limits(tmp_path: Path, fake_bin) -> None:
    events = turn(codex_request(tmp_path))

    assert [e["type"] for e in events] == ["session", "message", "usage_live", "usage", "limits", "adapter_done"]
    assert events[0]["session_id"] == "thread-fake-1" and events[1]["text"] == "eco: hola"
    # What the turn has used so far is reported as it grows, then the final figures replace it.
    assert events[2]["input_tokens"] == 120 and events[2]["output_tokens"] == 8
    # Tokens of this turn: the thread total minus what it had before the turn's first model call.
    assert events[3]["input_tokens"] == 120 and events[3]["output_tokens"] == 8
    assert events[4]["context_tokens"] == 120 and events[4]["context_window"] == 200000
    assert events[-1] == {**events[-1], "session_id": "thread-fake-1", "exit_code": 0, "cancelled": False}
    assert all(e["schema"] == "chxchx.agent-event" and e["schema_version"] == 1 for e in events)


def test_the_settings_of_the_request_reach_the_thread_and_the_turn(tmp_path: Path, fake_bin) -> None:
    settings = {"sandbox": "read-only", "approvalPolicy": "never", "sandboxPolicy": {"type": "readOnly", "networkAccess": False}}
    turn(codex_request(tmp_path, codex=settings, model="gpt-x", effort="high"))

    thread = next(row for row in fake_bin("codex") if "thread" in row)
    started = next(row for row in fake_bin("codex") if "turn" in row)["turn"]
    assert thread["params"]["sandbox"] == "read-only" and thread["params"]["cwd"] == str(tmp_path)
    assert started["approvalPolicy"] == "never" and started["sandboxPolicy"] == settings["sandboxPolicy"]
    assert started["model"] == "gpt-x" and started["effort"] == "high"


def test_a_resumed_turn_uses_thread_resume(tmp_path: Path, fake_bin) -> None:
    turn(codex_request(tmp_path, session_id="thread-old"))

    row = next(row for row in fake_bin("codex") if "thread" in row)
    assert row["thread"] == "thread/resume" and row["params"]["threadId"] == "thread-old"


def test_only_the_configured_overrides_are_passed_to_the_server(tmp_path: Path, fake_bin) -> None:
    turn(codex_request(tmp_path, argv=["codex", "app-server", "-c", "model_provider=x"]))

    assert fake_bin("codex")[0]["argv"] == ["app-server", "-c", "model_provider=x"]


@pytest.mark.parametrize("scope,decision", [(None, "accept"), ("session", "acceptForSession")])
def test_allowing_a_command_answers_the_request_and_continues(tmp_path: Path, scope, decision) -> None:
    events = turn(codex_request(tmp_path, "PERMISO"), allow(scope))

    card = _card(events)
    assert card["live"] and card["tool"] == "Bash" and "python3 -V" in card["summary"] and card["detail"] == "version"
    assert [choice.get("scope") for choice in card["choices"]] == [None, "session"]
    assert texts(events)[-1] == f'decision: "{decision}"'


def test_denying_declines_and_the_agent_carries_on(tmp_path: Path) -> None:
    assert texts(turn(codex_request(tmp_path, "PERMISO"), deny))[-1] == 'decision: "decline"'


def test_file_changes_and_extra_permissions(tmp_path: Path) -> None:
    events = turn(codex_request(tmp_path, "ARCHIVO"), allow())
    assert _card(events)["summary"].startswith("Modificar archivos") and texts(events)[-1] == 'decision: "accept"'

    events = turn(codex_request(tmp_path, "RED"), allow("session"))
    assert "red" in _card(events)["summary"]
    assert json.loads(texts(events)[-1].removeprefix("permisos: ")) == {"permissions": {"network": {"enabled": True}}, "scope": "session"}

    events = turn(codex_request(tmp_path, "RED"), deny)
    assert json.loads(texts(events)[-1].removeprefix("permisos: ")) == {"permissions": {}, "scope": "turn"}


def test_questions_map_answers_back_to_codex_ids(tmp_path: Path) -> None:
    events = turn(codex_request(tmp_path, "PREGUNTA"),
                  lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {"Color?": "Azul"}})
    ask = next(e for e in events if e["type"] == "question")
    assert ask["questions"][0]["question"] == "Color?" and [o["label"] for o in ask["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert json.loads(texts(events)[-1].removeprefix("respuesta: ")) == {"color": {"answers": ["Azul"]}}

    events = turn(codex_request(tmp_path, "PREGUNTA"), lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})
    assert json.loads(texts(events)[-1].removeprefix("respuesta: ")) == {}


def test_command_items_become_tool_cards(tmp_path: Path) -> None:
    tool = next(e for e in turn(codex_request(tmp_path, "PERMISO"), allow()) if e["type"] == "tool")
    assert tool["kind"] == "command" and tool["title"] == "python3 -V" and tool["status"] == "completed" and "3.11" in tool["output"]


def test_cancel_stops_a_turn_that_waits_and_is_not_a_failure(tmp_path: Path) -> None:
    events = turn(codex_request(tmp_path, "PERMISO"), lambda e: "cancel")

    assert events[-1]["type"] == "adapter_done" and events[-1]["cancelled"] is True
    assert not [e for e in events if e["type"] in {"message", "error"}]


def test_closing_the_hosts_input_stops_a_turn_that_waits(tmp_path: Path) -> None:
    events = turn(codex_request(tmp_path, "PERMISO"), lambda e: "close")

    assert events[-1]["type"] == "adapter_done" and events[-1]["cancelled"] is True
    assert not [e for e in events if e["type"] == "message"]


def test_a_missing_program_is_reported_with_the_exit_code_of_not_found(tmp_path: Path) -> None:
    events = turn(codex_request(tmp_path, argv=["codex", "app-server"], env={"PATH": str(tmp_path)}))

    assert events[0]["type"] == "error" and "codex" in events[0]["message"]
    assert events[-1]["exit_code"] == 127


def test_usage_adds_every_model_call_of_the_turn_but_not_the_threads_past() -> None:
    live = LiveCodex(io.StringIO(), cwd=Path("."), settings={"sandbox": "workspace-write", "approvalPolicy": "never",
                     "sandboxPolicy": {}}, session_id=None, model=None, effort=None)
    for total, last in ((5_120, 120), (5_300, 180)):  # the thread already had 5_000 tokens before this turn
        live.handle({"method": "thread/tokenUsage/updated", "params": {"tokenUsage": {
            "total": {"inputTokens": total, "outputTokens": 0}, "last": {"inputTokens": last, "outputTokens": 0}, "modelContextWindow": 9}}})
    events = live.handle({"method": "turn/completed", "params": {"turn": {"status": "completed"}}})
    assert events[0] == {"type": "usage", "input_tokens": 300, "output_tokens": 0, "cost_usd": None}
    assert events[1]["context_tokens"] == 180 and live.done


def test_each_usage_update_is_also_reported_live_without_the_threads_past() -> None:
    live = LiveCodex(io.StringIO(), cwd=Path("."), settings={"sandbox": "workspace-write", "approvalPolicy": "never",
                     "sandboxPolicy": {}}, session_id=None, model=None, effort=None)
    seen = []
    for total, out, last, last_out in ((5_120, 10, 120, 10), (5_300, 40, 180, 30)):  # 5_000 tokens of earlier turns
        seen += live.handle({"method": "thread/tokenUsage/updated", "params": {"tokenUsage": {
            "total": {"inputTokens": total, "outputTokens": out},
            "last": {"inputTokens": last, "outputTokens": last_out}, "modelContextWindow": 9}}})
    assert seen == [
        {"type": "usage_live", "message_id": "turn", "input_tokens": 120, "output_tokens": 10},
        {"type": "usage_live", "message_id": "turn", "input_tokens": 300, "output_tokens": 40},
    ]
