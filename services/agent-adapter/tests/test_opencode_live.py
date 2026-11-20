"""OpenCode's live approvals and questions end to end, against a real fake `opencode serve` (HTTP + SSE)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from conftest import allow, deny, opencode_request, texts, turn

from chxchx_agent_adapter import opencode_live
from chxchx_agent_adapter.opencode_stream import normalize_opencode
from chxchx_agent_adapter.permissions import opencode_denial


@pytest.fixture(params=["1.19.5", "2.2.6"], ids=["v1", "v2"])
def version(request, monkeypatch) -> str:
    monkeypatch.setenv("FAKE_OPENCODE_VERSION", request.param)
    return request.param


def _request(root: Path, version: str, prompt: str = "hola", **overrides):
    major = int(version.split(".")[0])
    return opencode_request(root, prompt, opencode_major=major, **overrides)


def _server(fake_bin, version: str) -> list[dict]:
    return fake_bin("opencode-serve-v2" if version.startswith("2") else "opencode-serve")


def test_a_plain_turn_streams_session_message_and_usage(tmp_path: Path, fake_bin, version) -> None:
    events = turn(_request(tmp_path, version))

    kinds = [e["type"] for e in events]
    assert kinds[0] == "session" and kinds[-1] == "adapter_done" and "message" in kinds and "usage" in kinds
    assert "eco: hola" in texts(events) and events[-1]["exit_code"] == 0
    assert not any(e["type"] == "error" for e in events)
    assert _server(fake_bin, version)[0]["has_password"]  # the server is never left unprotected


def test_the_prompt_carries_agent_model_and_effort(tmp_path: Path, fake_bin) -> None:
    version = "1.19.5"
    turn(_request(tmp_path, version, model="ollama/qwen3:8b", effort="high"))
    turn(_request(tmp_path, version, opencode_agent="plan"))

    prompts = [row["prompt"] for row in _server(fake_bin, version) if "prompt" in row]
    assert prompts[0]["agent"] == "build" and prompts[0]["model"] == {"providerID": "ollama", "modelID": "qwen3:8b"}
    assert prompts[1]["agent"] == "plan" and "model" not in prompts[1]


def test_the_prompt_carries_the_image_as_a_file_part(tmp_path: Path, fake_bin) -> None:
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")

    turn(_request(tmp_path, "1.19.5", "Describe la captura", images=[image]))

    prompt = next(row["prompt"] for row in _server(fake_bin, "1.19.5") if "prompt" in row)
    part = next(part for part in prompt["parts"] if part["type"] == "file")
    assert part["mime"] == "image/png" and part["filename"].endswith("captura.png") and image.as_uri() == part["url"]


def test_version_two_receives_the_text_and_the_image_as_files(tmp_path: Path, fake_bin, monkeypatch) -> None:
    monkeypatch.setenv("FAKE_OPENCODE_VERSION", "2.2.6")
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")

    turn(_request(tmp_path, "2.2.6", "Describe la captura", images=[image]))

    prompt = next(row["prompt"] for row in _server(fake_bin, "2.2.6") if "prompt" in row)
    assert prompt["text"] == "Describe la captura"
    assert [(f["name"], f["uri"], f["description"]) for f in prompt["files"]] == [("captura.png", image.as_uri(), "image/png")]


def test_the_environment_of_the_request_reaches_the_server(tmp_path: Path, fake_bin) -> None:
    policy = {"bash": {"*": "ask", "python3 *": "allow"}}
    turn(opencode_request(tmp_path, env={"OPENCODE_PERMISSION": json.dumps(policy)}))

    assert json.loads(_server(fake_bin, "1.19.5")[0]["permission"]) == policy


def test_allowing_replies_once_and_the_turn_continues(tmp_path: Path, fake_bin) -> None:
    events = turn(opencode_request(tmp_path, "PERMISO"), allow())

    card = next(e for e in events if e["type"] == "permission")
    assert card["live"] and card["tool"] == "bash" and "python3 -V" in card["summary"]
    assert [(c["label"], c["allow"], c["persist"]) for c in card["choices"]] == [
        ("Permitir esta vez", [], False), ("Permitir «python3 *» en este chat", ["bash:python3 *"], True)]
    assert texts(events) == ['decision: "once"']
    reply = next(row for row in _server(fake_bin, "1.19.5") if "reply_to" in row)
    assert reply["reply_to"] == "per_1" and reply["body"] == {"reply": "once"}
    tool = next(e for e in events if e["type"] == "tool")
    assert tool["status"] == "completed" and "python3 -V" in tool["title"]


def test_denying_rejects_the_request(tmp_path: Path) -> None:
    events = turn(opencode_request(tmp_path, "PERMISO"), deny)

    assert texts(events) == ['decision: "reject"']
    # The rejected tool call is already shown through the live card: no duplicate "blocked" card is built.
    assert len([e for e in events if e["type"] == "permission"]) == 1


def test_questions_are_answered_by_position(tmp_path: Path) -> None:
    events = turn(opencode_request(tmp_path, "PREGUNTA"),
                  lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {"Color?": "Azul"}})

    ask = next(e for e in events if e["type"] == "question")
    assert ask["questions"][0]["question"] == "Color?" and [o["label"] for o in ask["questions"][0]["options"]] == ["Rojo", "Azul"]
    assert texts(events) == ['respuesta: [["Azul"]]']


def test_skipping_a_question_rejects_it(tmp_path: Path) -> None:
    events = turn(opencode_request(tmp_path, "PREGUNTA"), lambda e: {"type": "answer", "request_id": e["request_id"], "answers": {}})

    assert texts(events) == ['respuesta: "rejected"']


def test_a_missing_session_is_reported_instead_of_hanging(tmp_path: Path) -> None:
    events = turn(opencode_request(tmp_path, session_id="ses_gone"))

    assert [e["type"] for e in events][-2:] == ["error", "adapter_done"] and "servidor de OpenCode" in events[-2]["message"]


def test_cancelling_a_waiting_turn_ends_it_without_an_error(tmp_path: Path) -> None:
    events = turn(opencode_request(tmp_path, "PERMISO"), lambda e: "cancel")

    assert events[-1]["type"] == "adapter_done" and events[-1]["cancelled"] is True
    assert not any(e["type"] == "error" for e in events)


def test_waiting_for_the_person_is_not_a_stall(tmp_path: Path, monkeypatch) -> None:
    """A card left unanswered longer than the stall limit must not end the turn with "dejó de responder"."""
    monkeypatch.setattr(opencode_live, "_STALL_SECONDS", 2)

    events = turn(opencode_request(tmp_path, "PERMISO por favor"), lambda e: (4.5, allow()(e)))

    assert not any(e["type"] == "error" for e in events)
    assert events[-1]["type"] == "adapter_done" and events[-1]["exit_code"] == 0


def test_the_local_server_is_never_reached_through_a_proxy(tmp_path: Path, monkeypatch) -> None:
    """A user's HTTP(S)_PROXY must not capture the conversation with the local server."""
    for name in ("HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"):
        monkeypatch.setenv(name, "http://127.0.0.1:9")  # nothing listens there
    monkeypatch.delenv("NO_PROXY", raising=False)
    monkeypatch.delenv("no_proxy", raising=False)

    events = turn(opencode_request(tmp_path))

    assert events[-1]["exit_code"] == 0 and "eco: hola" in texts(events)


def test_a_rejected_request_of_version_two_becomes_a_permission_card() -> None:
    record = {"type": "tool_use", "sessionID": "s", "part": {"type": "tool", "callID": "c9", "tool": "shell", "state": {
        "status": "error", "input": {"command": "echo hola"},
        "error": "This non-interactive run cannot ask the user for permission, so the request was rejected. Continue without this action."}}}

    events = normalize_opencode(record)

    assert [e["type"] for e in events] == ["tool", "permission"] and events[0]["kind"] == "command"
    assert events[1]["tool"] == "shell" and events[1]["choices"][0]["allow"] == ["bash"]
    assert not opencode_denial({"tool": "shell", "state": {"status": "error", "error": "no such file"}}, "shell: ls")
