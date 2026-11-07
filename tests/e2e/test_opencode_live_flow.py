"""OpenCode's live approvals and questions end to end: a real fake `opencode serve` (HTTP + SSE) that waits for the host."""

from __future__ import annotations

import json
import os
import threading
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.workspace import agent_chat


def _service(root: Path):
    agent = SimpleNamespace(id="opencode", command=["opencode"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin) -> None:
    pass


def _turn(root: Path, prompt: str, answer, **options):
    options.setdefault("mode", "workspace-write")
    options.setdefault("session_id", None)
    plan = agent_chat.plan_chat_turn(_service(root), "opencode", live=True, **options)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    events: list[dict] = []

    def emit(event: dict) -> None:
        events.append(event)
        if event["type"] in {"permission", "question"} and event.get("request_id"):
            reply = answer(event)
            if reply is not None:
                writer.write(json.dumps(reply) + "\n")
                writer.flush()

    done = threading.Event()
    threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, prompt, emit, cancel_input=reader), done.set()), daemon=True).start()
    if not done.wait(30):
        import shutil
        raise AssertionError(f"the turn never finished; argv={plan.argv} which={shutil.which('opencode')} events={events} server={_rows_safe()}")
    writer.close()
    return plan, events


def _texts(events):
    return [event["text"] for event in events if event["type"] == "message"]


def _allow(event):
    return {"type": "answer", "request_id": event["request_id"], "behavior": "allow"}


def _deny(event):
    return {"type": "answer", "request_id": event["request_id"], "behavior": "deny"}


def _rows_safe():
    log = os.environ.get("FAKE_CLI_LOG")
    try:
        return Path(log).read_text(encoding="utf-8")[-1500:] if log else "(sin registro)"
    except OSError:
        return "(registro ilegible)"


def _rows(fake_bin, tool="opencode-serve"):
    return fake_bin(tool)


def test_a_plain_turn_streams_session_message_and_usage(tmp_path: Path, fake_bin) -> None:
    plan, events = _turn(tmp_path, "hola", lambda e: None)
    assert plan.live and plan.argv[1:] == ["serve", "--port", "0", "--hostname", "127.0.0.1"]
    assert [e["type"] for e in events] == ["start", "session", "message", "usage", "limits", "done"]
    assert events[1]["session_id"] == "ses_fake_live" and events[2]["text"] == "eco: hola"
    assert events[3]["input_tokens"] == 150 and events[3]["output_tokens"] == 7
    assert events[-1]["exit_code"] == 0 and not any(e["type"] == "error" for e in events)
    server = _rows(fake_bin)[0]
    assert server["argv"][0] == "serve" and server["has_password"]  # the server is never left unprotected


def test_the_prompt_carries_agent_model_and_effort(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, "hola", lambda e: None, model="ollama/qwen3:8b", effort="high")
    _turn(tmp_path, "hola", lambda e: None, mode="read-only")
    prompts = [row["prompt"] for row in _rows(fake_bin) if "prompt" in row]
    assert prompts[0]["agent"] == "build" and prompts[0]["model"] == {"providerID": "ollama", "modelID": "qwen3:8b"} and prompts[0]["variant"] == "high"
    assert prompts[1]["agent"] == "plan" and "model" not in prompts[1]


def test_the_permission_policy_is_given_to_the_server(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, "hola", lambda e: None, allow=("bash:python3 *",))
    server = _rows(fake_bin)[0]
    assert json.loads(server["permission"])["bash"] == {"*": "ask", "python3 *": "allow"}


def test_allowing_replies_once_and_the_turn_continues(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PERMISO", _allow)
    card = next(e for e in events if e["type"] == "permission")
    assert card["live"] and card["tool"] == "bash" and "python3 -V" in card["summary"]
    assert [(c["label"], c["allow"], c["persist"]) for c in card["choices"]] == [
        ("Permitir esta vez", [], False), ("Permitir «python3 *» en este chat", ["bash:python3 *"], True)]
    assert _texts(events) == ['decision: "once"']
    reply = [row for row in _rows(fake_bin) if "reply_to" in row][0]
    assert reply["reply_to"] == "per_1" and reply["body"] == {"reply": "once"}
    tool = next(e for e in events if e["type"] == "tool")
    assert tool["status"] == "completed" and "python3 -V" in tool["title"]


def test_denying_rejects_the_request(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PERMISO", _deny)
    assert _texts(events) == ['decision: "reject"']
    # The rejected tool call is already shown through the live card: no duplicate "blocked" card is built.
    assert len([e for e in events if e["type"] == "permission"]) == 1


def test_questions_are_answered_by_position(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {"Color?": "Azul"}})
    ask = next(e for e in events if e["type"] == "question")
    assert ask["questions"][0]["question"] == "Color?" and [o["label"] for o in ask["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert _texts(events) == ['respuesta: [["Azul"]]']


def test_skipping_a_question_rejects_it(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})
    assert _texts(events) == ['respuesta: "rejected"']


def test_a_missing_session_is_reported_instead_of_hanging(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "hola", lambda e: None, session_id="ses_gone")
    assert [e["type"] for e in events][-2:] == ["error", "done"] and "servidor de OpenCode" in events[-2]["message"]


def test_cancelling_a_waiting_turn_ends_it_without_an_error(tmp_path: Path) -> None:
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "opencode", mode="workspace-write", session_id=None, live=True)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r"), os.fdopen(write_end, "w")
    events: list[dict] = []

    def emit(event: dict) -> None:
        events.append(event)
        if event["type"] == "permission":
            writer.write("cancel\n")
            writer.flush()

    done = threading.Event()
    threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, "PERMISO", emit, cancel_input=reader), done.set()), daemon=True).start()
    assert done.wait(30)
    writer.close()
    assert events[-1]["type"] == "done" and events[-1]["cancelled"] is True
    assert not any(e["type"] == "error" for e in events)


def test_waiting_for_the_person_is_not_a_stall(tmp_path: Path, monkeypatch) -> None:
    """A card left unanswered longer than the stall limit must not end the turn with "dejó de responder"."""
    from chxchx_tech_lead.integrations import agent_opencode_live

    monkeypatch.setattr(agent_opencode_live, "_STALL_SECONDS", 2)
    # The fake server asks for a permission; nobody answers for twice the limit, then the turn is cancelled by closing stdin.
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "opencode", live=True, mode="workspace-write", session_id=None)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    events: list[dict] = []

    def emit(event: dict) -> None:
        events.append(event)
        if event["type"] == "permission":
            threading.Timer(4.5, lambda: (writer.write(json.dumps(_allow(event)) + "\n"), writer.flush())).start()

    done = threading.Event()
    threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, "PERMISO por favor", emit, cancel_input=reader), done.set()), daemon=True).start()
    assert done.wait(30)
    writer.close()
    assert not any(e["type"] == "error" for e in events)
    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 0


def test_the_local_server_is_never_reached_through_a_proxy(tmp_path: Path, monkeypatch) -> None:
    """A user's HTTP(S)_PROXY must not capture the conversation with the local server."""
    for name in ("HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"):
        monkeypatch.setenv(name, "http://127.0.0.1:9")  # nothing listens there
    monkeypatch.delenv("NO_PROXY", raising=False)
    monkeypatch.delenv("no_proxy", raising=False)

    _, events = _turn(tmp_path, "hola", lambda e: None)

    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 0 and "eco: hola" in _texts(events)
