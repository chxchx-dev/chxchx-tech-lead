"""Closing and reopening a visual chat: what was said, the session and the hand-offs all survive."""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.workspace import agent_chat
from chxchx_tech_lead.workspace.chat_state import load_state, mark_unverified, state_payload


def _service(root: Path):
    agent = SimpleNamespace(id="opencode", command=["opencode"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin, tmp_path: Path, monkeypatch) -> None:
    cache = tmp_path / "cache" / "opencode"
    cache.mkdir(parents=True)
    (cache / "models.json").write_text(json.dumps({"x": {"models": {"y": {"limit": {"context": 1000}}}}}), encoding="utf-8")
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "config"))


def _turn(root: Path, prompt: str, monkeypatch, *, tokens: int = 150, session_id: str | None = None, suffix: str = "", **options):
    monkeypatch.setenv("FAKE_CONTEXT_TOKENS", str(tokens))
    monkeypatch.setenv("FAKE_SESSION_SUFFIX", suffix)
    plan = agent_chat.plan_chat_turn(_service(root), "opencode", mode="workspace-write", session_id=session_id, model="x/y", **options)
    events: list[dict] = []
    code = agent_chat.run_chat_turn(plan, prompt, events.append)
    return plan, code, events


def _runs(fake_bin) -> list[dict]:
    return [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]]


def _texts(root: Path) -> list[tuple[str, str]]:
    return [(m["role"], m["text"]) for m in load_state(root, "opencode")["messages"]]


def test_every_turn_leaves_the_conversation_and_the_session_on_disk(tmp_path: Path, monkeypatch) -> None:
    _turn(tmp_path, "primera pregunta", monkeypatch)
    _turn(tmp_path, "segunda pregunta", monkeypatch, session_id="ses_fake1")

    saved = state_payload(tmp_path, "opencode")

    assert saved["session_id"] == "ses_fake1" and saved["provider"] == "OpenCode"
    assert [(m["role"], m["text"]) for m in saved["messages"]] == [
        ("user", "primera pregunta"), ("assistant", "eco: primera pregunta"),
        ("user", "segunda pregunta"), ("assistant", "eco: segunda pregunta")]


def test_the_persons_message_survives_a_turn_that_never_finished(tmp_path: Path, monkeypatch) -> None:
    _, code, _ = _turn(tmp_path, "FALLA por favor", monkeypatch)

    assert code == 1 and _texts(tmp_path)[0] == ("user", "FALLA por favor")


def test_one_off_questions_and_the_hand_off_turn_are_not_part_of_the_conversation(tmp_path: Path, monkeypatch) -> None:
    _turn(tmp_path, "pregunta suelta", monkeypatch, lean=True)
    assert load_state(tmp_path, "opencode") is None

    _turn(tmp_path, "hola", monkeypatch, tokens=950)                   # releases the context: a hand-off turn runs
    assert _texts(tmp_path) == [("user", "hola"), ("assistant", "eco: hola")]   # the hand-off prompt/answer are not stored


def test_reopening_a_chat_whose_session_still_exists_resumes_it(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _turn(tmp_path, "hola", monkeypatch)
    mark_unverified(tmp_path, "opencode")                               # what Studio does when it restores the chat

    plan, code, _ = _turn(tmp_path, "sigue", monkeypatch, session_id="ses_fake1")

    run = _runs(fake_bin)[-1]
    assert code == 0 and plan.recap == "" and run["argv"][run["argv"].index("--session") + 1] == "ses_fake1"
    assert load_state(tmp_path, "opencode")["verified"] is True        # checked once, not on every turn


def test_reopening_a_chat_whose_session_is_gone_rebuilds_it_from_what_was_saved(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _turn(tmp_path, "Quiero migrar el instalador", monkeypatch, session_id="ses_dead")   # saved under a session the CLI no longer lists
    mark_unverified(tmp_path, "opencode")

    plan, code, events = _turn(tmp_path, "¿en qué íbamos?", monkeypatch, session_id="ses_dead", suffix="b")

    run = _runs(fake_bin)[-1]
    assert code == 0 and plan.session_id is None and plan.recap
    assert "--session" not in run["argv"]
    assert "Persona: Quiero migrar el instalador" in run["stdin"] and run["stdin"].rstrip().endswith("¿en qué íbamos?")
    assert events[-1]["session_id"] == "ses_fake1b"
    assert state_payload(tmp_path, "opencode")["session_id"] == "ses_fake1b"                  # and it carries on from the new one
    assert _texts(tmp_path)[-2:] == [("user", "¿en qué íbamos?"), ("assistant", "eco: ¿en qué íbamos?")]


def test_a_released_context_survives_closing_before_the_next_message(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _turn(tmp_path, "primera", monkeypatch, tokens=950)               # hand-off saved for ses_fake1
    reopened = state_payload(tmp_path, "opencode", restoring=True)    # Studio closed and opened again
    assert reopened["session_id"] == "ses_fake1" and len(reopened["messages"]) == 2

    plan, _, _ = _turn(tmp_path, "segunda", monkeypatch, tokens=120, session_id=reopened["session_id"], suffix="b")

    run = _runs(fake_bin)[-1]
    assert plan.continued_from == "ses_fake1" and "--session" not in run["argv"]
    assert "TRASPASO: migrar el instalador" in run["stdin"] and run["stdin"].rstrip().endswith("segunda")
    assert state_payload(tmp_path, "opencode")["session_id"] == "ses_fake1b"
    assert not list((tmp_path / ".ai" / "chat-handoffs").glob("*.md"))        # consumed only now that the new session exists
