"""Claude's live approvals and questions end to end: a real fake `claude` process on PATH that waits for the host."""

from __future__ import annotations

import json
import os
import threading
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.workspace import agent_chat


def _service(root: Path):
    agent = SimpleNamespace(id="claude", command=["claude"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin) -> None:
    pass


def _turn(root: Path, prompt: str, answer, *, live: bool = True, **options):
    """Runs one turn; `answer(event)` returns the JSON line Studio would write for a live request."""
    options.setdefault("mode", "workspace-write")
    plan = agent_chat.plan_chat_turn(_service(root), "claude", session_id=None, live=live, **options)
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    events: list[dict] = []

    def emit(event: dict) -> None:
        events.append(event)
        if event["type"] in {"permission", "question"} and event.get("request_id"):
            reply = answer(event)
            if reply == "close":  # Studio went away without answering
                writer.close()
            elif reply is not None:
                writer.write(json.dumps(reply) + "\n")
                writer.flush()

    done = threading.Event()
    thread = threading.Thread(target=lambda: (agent_chat.run_chat_turn(plan, prompt, emit, cancel_input=reader), done.set()), daemon=True)
    thread.start()
    assert done.wait(20), "the turn never finished"
    if not writer.closed:
        writer.close()
    return plan, events


def _texts(events):
    return [event["text"] for event in events if event["type"] == "message"]


def test_a_live_turn_asks_and_continues_when_allowed(tmp_path: Path, fake_bin) -> None:
    plan, events = _turn(tmp_path, "PERMISO", lambda e: {"type": "answer", "request_id": e["request_id"], "behavior": "allow"})

    card = next(event for event in events if event["type"] == "permission")
    assert card["live"] and card["tool"] == "Bash" and "python3 -V" in card["summary"]
    assert [choice["persist"] for choice in card["choices"]] == [False, True]
    assert _texts(events) == ["hecho"]
    call = fake_bin("claude")[0]
    assert "--input-format" in call["argv"] and call["argv"][call["argv"].index("--permission-prompt-tool") + 1] == "stdio"
    # The prompt travelled as a stream-json user message, and the answer went back as a control_response.
    assert "PERMISO" in call["prompt"]
    answered = [row["response"] for row in fake_bin("claude") if "response" in row][0]
    assert answered["behavior"] == "allow" and answered["updatedInput"]["command"] == "python3 -V"
    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 0


def test_denying_tells_the_agent_and_it_carries_on(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PERMISO", lambda e: {"type": "answer", "request_id": e["request_id"], "behavior": "deny"})
    assert _texts(events)[0].startswith("denegado: El usuario rechazó")


def test_questions_carry_their_options_and_the_answer(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {"Color?": "Azul"}})
    ask = next(event for event in events if event["type"] == "question")
    assert ask["questions"][0]["question"] == "Color?" and [o["label"] for o in ask["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert _texts(events) == ['respuesta: {"Color?": "Azul"}']


def test_skipping_a_question_denies_it(tmp_path: Path, fake_bin) -> None:
    _, events = _turn(tmp_path, "PREGUNTA", lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})
    assert "no respondió" in _texts(events)[0]


def test_closing_the_hosts_stdin_stops_a_turn_that_waits_for_an_answer(tmp_path: Path) -> None:
    _, events = _turn(tmp_path, "PERMISO", lambda e: "close")
    # Nobody answered the request: closing the host's stdin ends the turn instead of hanging.
    assert events[-1]["type"] == "done" and not _texts(events)


def test_without_live_the_prompt_is_plain_text_and_nothing_is_asked(tmp_path: Path, fake_bin) -> None:
    plan, events = _turn(tmp_path, "hola", lambda e: None, live=False)
    assert not plan.live and "--input-format" not in plan.argv
    assert not [event for event in events if event["type"] == "permission"]


def test_lean_turns_are_never_live(tmp_path: Path) -> None:
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "claude", mode="read-only", session_id=None, live=True, lean=True)
    assert not plan.live


def test_a_new_chat_carries_the_context_files_and_a_resumed_one_does_not(tmp_path: Path, fake_bin) -> None:
    (tmp_path / ".ai").mkdir()
    (tmp_path / ".ai" / "CURRENT_STATE.md").write_text("Estado: migrando a Qt\n", encoding="utf-8")
    _turn(tmp_path, "hola", lambda e: None)
    resumed = agent_chat.plan_chat_turn(_service(tmp_path), "claude", mode="workspace-write", session_id="claude-live-1")
    agent_chat.run_chat_turn(resumed, "otra cosa", lambda e: None)

    first, second = [row["prompt"] for row in fake_bin("claude") if "prompt" in row]
    assert '<archivo-de-contexto ruta=".ai/CURRENT_STATE.md">' in first and "migrando a Qt" in first
    assert first.rstrip().endswith("hola") and "No ejecutes comandos" in first
    assert "archivo-de-contexto" not in second and second.strip() == "otra cosa"
