"""OpenCode 2.x live approvals and questions end to end: a fake `opencode serve` with the 2.x `/api` and events."""

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
def machine(isolated_home, fake_bin, monkeypatch) -> None:
    monkeypatch.setenv("FAKE_OPENCODE_VERSION", "2.2.6")


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
    assert done.wait(30), f"the turn never finished: {[e['type'] for e in events]}"
    writer.close()
    return plan, events


def _rows(fake_bin) -> list[dict]:
    return fake_bin("opencode-serve-v2")


def _answer(behavior):
    return lambda event: {"type": "answer", "request_id": event["request_id"], "behavior": behavior}


def test_a_plain_turn_goes_through_the_v2_api(tmp_path: Path, fake_bin) -> None:
    plan, events = _turn(tmp_path, "hola", lambda e: None, model="x/y", effort="high")

    assert plan.live and plan.opencode_major == 2 and plan.argv[1:] == ["serve", "--port", "0", "--hostname", "127.0.0.1"]
    assert [e["type"] for e in events] == ["start", "session", "message", "usage", "limits", "done"]
    assert events[1]["session_id"] == "ses_fake_v2" and events[2]["text"] == "eco: hola"
    assert events[3] == {"type": "usage", "input_tokens": 150, "output_tokens": 7, "cost_usd": 0.001}
    server = _rows(fake_bin)
    assert server[0]["has_password"] and server[0]["config"] is not None                   # access level + password reach the server
    create = next(row["create"] for row in server if "create" in row)
    assert create["agent"] == "build" and create["model"] == {"id": "y", "providerID": "x", "variant": "high"}
    assert create["location"] == {"directory": str(tmp_path.resolve())}


def test_live_prompt_carries_the_staged_image_file(tmp_path: Path, fake_bin) -> None:
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")

    _turn(tmp_path, "Describe la captura", lambda e: None, images=(image,))

    prompt = next(row["prompt"] for row in _rows(fake_bin) if "prompt" in row)
    image_file = prompt["files"][0]
    staged = next((tmp_path / ".ai" / "chat-attachments").glob("*captura.png"))
    assert image_file["description"] == "image/png" and image_file["name"].endswith("captura.png")
    assert image_file["source"]["uri"] == image_file["uri"]
    assert staged.as_uri() == image_file["uri"] and staged.read_bytes() == b"png"


def test_allowing_replies_once_and_the_turn_continues(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PERMISO", _answer("allow"))

    card = next(e for e in events if e["type"] == "permission")
    assert card["live"] and card["tool"] == "bash" and "python3 -V" in card["summary"] and card["id"] == "call_1"
    assert [c["label"] for c in card["choices"]] == ["Permitir esta vez", "Permitir «python3 *» en este chat"]
    assert card["choices"][1]["allow"] == ["bash:python3 *"] and card["choices"][1]["persist"]
    tools = [e for e in events if e["type"] == "tool"]
    assert [(t["status"], t["kind"]) for t in tools] == [("running", "command"), ("completed", "command")] and tools[1]["output"] == "Python 3.12\n"
    assert next(row for row in _rows(fake_bin) if "permission_reply" in row)["body"] == {"decision": "once"}
    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 0 and not any(e["type"] == "error" for e in events)


def test_denying_rejects_and_a_refusal_is_not_shown_as_an_error(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PERMISO", _answer("deny"))

    assert next(row for row in _rows(fake_bin) if "permission_reply" in row)["body"] == {"decision": "reject"}
    assert [e["status"] for e in events if e["type"] == "tool"] == ["running", "failed"]
    assert not any(e["type"] == "error" for e in events) and events[-1]["exit_code"] == 0


def test_questions_are_answered_by_field_key_with_the_chosen_values(tmp_path: Path, fake_bin) -> None:
    def pick(event):
        return {"type": "answer", "request_id": event["request_id"], "answers": {"¿Cuál color?": "Azul", "¿Cuáles letras?": "A, B"}}

    _, events = _turn(tmp_path, "PREGUNTA", pick)

    card = next(e for e in events if e["type"] == "question")
    assert [(q["question"], q["header"], q["multiSelect"]) for q in card["questions"]] == [("¿Cuál color?", "Color", False), ("¿Cuáles letras?", "Letras", True)]
    assert [o["label"] for o in card["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert next(row for row in _rows(fake_bin) if "form_reply" in row)["body"] == {"answer": {"q0": "Azul", "q1": ["a", "b"]}}
    assert [e["text"] for e in events if e["type"] == "message"] == ['respuesta: {"q0": "Azul", "q1": ["a", "b"]}']


def test_skipping_a_question_cancels_the_form(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})

    assert any("skipped_form" in row for row in _rows(fake_bin)) and events[-1]["type"] == "done"


def test_resuming_a_session_checks_it_and_switches_agent_and_model(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "sigue", lambda e: None, session_id="ses_old", mode="read-only", model="x/y")

    rows = _rows(fake_bin)
    assert not any("create" in row for row in rows)
    assert [(row["switch"], row["body"]) for row in rows if "switch" in row] == [("agent", {"agent": "plan"}), ("model", {"model": {"id": "y", "providerID": "x"}})]
    assert events[1]["session_id"] == "ses_old" and events[-1]["exit_code"] == 0


def test_a_missing_session_is_reported_instead_of_hanging(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "hola", lambda e: None, session_id="ses_gone")

    assert [e["type"] for e in events][-2:] == ["error", "done"] and "servidor de OpenCode" in events[-2]["message"]


def test_cancelling_a_waiting_turn_ends_it_without_an_error(tmp_path: Path) -> None:
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "opencode", mode="workspace-write", session_id=None, live=True)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    seen: list[dict] = []

    def emit(event: dict) -> None:
        seen.append(event)
        if event["type"] == "permission":
            writer.write("cancel\n")
            writer.flush()

    done = threading.Event()
    threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, "PERMISO", emit, cancel_input=reader), done.set()), daemon=True).start()
    assert done.wait(30)
    writer.close()
    assert seen[-1]["type"] == "done" and seen[-1]["cancelled"] is True and not any(e["type"] == "error" for e in seen)
