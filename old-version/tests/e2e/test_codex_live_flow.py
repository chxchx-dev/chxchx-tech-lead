"""Codex's live approvals and questions end to end: a real fake `codex app-server` process that waits for the host."""

from __future__ import annotations

import json
import os
import threading
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.workspace import agent_chat


def _service(root: Path, command=None):
    agent = SimpleNamespace(id="codex", command=command or ["codex"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin) -> None:
    pass


def _turn(root: Path, prompt: str, answer, *, command=None, **options):
    options.setdefault("mode", "workspace-write")
    options.setdefault("session_id", None)
    plan = agent_chat.plan_chat_turn(_service(root, command), "codex", live=True, **options)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    events: list[dict] = []

    def emit(event: dict) -> None:
        events.append(event)
        if event["type"] in {"permission", "question"} and event.get("request_id"):
            reply = answer(event)
            if reply == "close":
                writer.close()
            elif reply is not None:
                writer.write(json.dumps(reply) + "\n")
                writer.flush()

    done = threading.Event()
    threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, prompt, emit, cancel_input=reader), done.set()), daemon=True).start()
    assert done.wait(20), "the turn never finished"
    if not writer.closed:
        writer.close()
    return plan, events


def _text(events):
    return [event["text"] for event in events if event["type"] == "message"][-1]


def _allow(scope=None):
    def reply(event):
        return {"type": "answer", "request_id": event["request_id"], "behavior": "allow", **({"scope": scope} if scope else {})}
    return reply


def test_a_plain_turn_streams_session_message_usage_and_limits(tmp_path: Path, fake_bin) -> None:
    plan, events = _turn(tmp_path, "hola", lambda e: None)
    assert plan.live and plan.argv[1:] == ["app-server"]
    assert [e["type"] for e in events] == ["start", "session", "message", "usage", "limits", "done"]
    assert events[1]["session_id"] == "thread-fake-1" and events[2]["text"] == "eco: hola"
    # Tokens of this turn: the thread total minus what it had before the turn's first model call.
    assert events[3]["input_tokens"] == 120 and events[3]["output_tokens"] == 8
    assert events[4]["context_tokens"] == 120 and events[4]["context_window"] == 200000
    assert events[-1]["session_id"] == "thread-fake-1" and events[-1]["exit_code"] == 0


def test_the_access_level_becomes_the_sandbox_and_approval_policy(tmp_path: Path, fake_bin) -> None:
    for mode, network in (("workspace-write", True), ("workspace-commands", False), ("read-only", False), ("full-access", False)):
        _turn(tmp_path, "hola", lambda e: None, mode=mode, network=network, model="gpt-x", effort="high")
    calls = [row for row in fake_bin("codex") if "turn" in row]
    threads = [row for row in fake_bin("codex") if "thread" in row]
    assert [t["params"]["sandbox"] for t in threads] == ["workspace-write", "workspace-write", "read-only", "danger-full-access"]
    assert [c["turn"]["approvalPolicy"] for c in calls] == ["on-request", "never", "never", "never"]
    assert calls[0]["turn"]["sandboxPolicy"] == {"type": "workspaceWrite", "networkAccess": True}
    assert calls[1]["turn"]["sandboxPolicy"]["networkAccess"] is False
    assert calls[3]["turn"]["sandboxPolicy"] == {"type": "dangerFullAccess"}
    assert calls[0]["turn"]["model"] == "gpt-x" and calls[0]["turn"]["effort"] == "high"


def test_a_resumed_turn_uses_thread_resume(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, "hola", lambda e: None, session_id="thread-old")
    row = [r for r in fake_bin("codex") if "thread" in r][0]
    assert row["thread"] == "thread/resume" and row["params"]["threadId"] == "thread-old"


@pytest.mark.parametrize("scope,decision", [(None, "accept"), ("session", "acceptForSession")])
def test_allowing_a_command_answers_the_request_and_continues(tmp_path: Path, scope, decision) -> None:
    _, events = _turn(tmp_path, "PERMISO", _allow(scope))
    card = next(e for e in events if e["type"] == "permission")
    assert card["live"] and card["tool"] == "Bash" and "python3 -V" in card["summary"] and card["detail"] == "version"
    assert [choice.get("scope") for choice in card["choices"]] == [None, "session"]
    assert _text(events) == f'decision: "{decision}"'


def test_denying_declines_and_the_agent_carries_on(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "PERMISO", lambda e: {"type": "answer", "request_id": e["request_id"], "behavior": "deny"})
    assert _text(events) == 'decision: "decline"'


def test_file_changes_and_extra_permissions(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "ARCHIVO", _allow())
    assert next(e for e in events if e["type"] == "permission")["summary"].startswith("Modificar archivos")
    assert _text(events) == 'decision: "accept"'
    _, events = _turn(tmp_path, "RED", _allow("session"))
    card = next(e for e in events if e["type"] == "permission")
    assert "red" in card["summary"]
    assert json.loads(_text(events).removeprefix("permisos: ")) == {"permissions": {"network": {"enabled": True}}, "scope": "session"}
    _, events = _turn(tmp_path, "RED", lambda e: {"type": "answer", "request_id": e["request_id"], "behavior": "deny"})
    assert json.loads(_text(events).removeprefix("permisos: ")) == {"permissions": {}, "scope": "turn"}


def test_questions_map_answers_back_to_codex_ids(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {"Color?": "Azul"}})
    ask = next(e for e in events if e["type"] == "question")
    assert ask["questions"][0]["question"] == "Color?" and [o["label"] for o in ask["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert json.loads(_text(events).removeprefix("respuesta: ")) == {"color": {"answers": ["Azul"]}}
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})
    assert json.loads(_text(events).removeprefix("respuesta: ")) == {}


def test_command_items_become_tool_cards(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "PERMISO", _allow())
    tool = next(e for e in events if e["type"] == "tool")
    assert tool["kind"] == "command" and tool["title"] == "python3 -V" and tool["status"] == "completed" and "3.11" in tool["output"]


def test_closing_the_hosts_stdin_stops_a_turn_that_waits(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "PERMISO", lambda e: "close")
    assert events[-1]["type"] == "done" and not [e for e in events if e["type"] == "message"]


def test_configured_overrides_are_kept_and_other_flags_are_not(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, "hola", lambda e: None, command=["codex", "-c", "model_provider=x", "--search"])
    assert fake_bin("codex")[0]["argv"] == ["app-server", "-c", "model_provider=x"]


def test_usage_adds_every_model_call_of_the_turn_but_not_the_threads_past() -> None:
    import io

    from chxchx_tech_lead.integrations.agent_access import Access
    from chxchx_tech_lead.integrations.agent_codex_live import LiveCodex

    live = LiveCodex(io.StringIO(), cwd=Path("."), access=Access("workspace-write"), session_id=None, model=None, effort=None)
    for total, last in ((5_120, 120), (5_300, 180)):  # the thread already had 5_000 tokens before this turn
        live.handle({"method": "thread/tokenUsage/updated", "params": {"tokenUsage": {
            "total": {"inputTokens": total, "outputTokens": 0}, "last": {"inputTokens": last, "outputTokens": 0}, "modelContextWindow": 9}}})
    events = live.handle({"method": "turn/completed", "params": {"turn": {"status": "completed"}}})
    assert events[0] == {"type": "usage", "input_tokens": 300, "output_tokens": 0, "cost_usd": None}
    assert events[1]["context_tokens"] == 180 and live.done
