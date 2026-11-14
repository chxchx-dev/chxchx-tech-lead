"""A visual chat survives closing Studio: its session and conversation are saved, restored and, if need be, rebuilt."""

from __future__ import annotations

import json
import os
from pathlib import Path

from chxchx_tech_lead.workspace import chat_resume, chat_state
from chxchx_tech_lead.workspace.chat_compaction import load_handoff, save_handoff
from chxchx_tech_lead.workspace.chat_state import (
    begin_turn,
    end_turn,
    load_state,
    mark_unverified,
    recap_block,
    reset_state,
    state_path,
    state_payload,
)


def test_a_turn_is_saved_when_it_starts_and_when_it_ends(tmp_path: Path) -> None:
    begin_turn(tmp_path, "codex", "Codex", None, "¿Qué hace app.py?")
    assert [(m["role"], m["text"]) for m in load_state(tmp_path, "codex")["messages"]] == [("user", "¿Qué hace app.py?")]  # closed mid-turn

    end_turn(tmp_path, "codex", "Codex", "t-1", ["Imprime hola.", "Y nada más."])
    again = state_payload(tmp_path, "codex")

    assert again["session_id"] == "t-1" and again["provider"] == "Codex"
    assert [(m["role"], m["text"]) for m in again["messages"]] == [("user", "¿Qué hace app.py?"), ("assistant", "Imprime hola."), ("assistant", "Y nada más.")]
    assert state_path(tmp_path, "codex").is_relative_to(tmp_path / ".ai" / "chat-state")


def test_the_state_is_bounded_and_never_keeps_secrets(tmp_path: Path) -> None:
    for index in range(100):
        begin_turn(tmp_path, "claude", "Claude", "s", f"mensaje {index}")
    begin_turn(tmp_path, "claude", "Claude", "s", "x" * 20_000)
    begin_turn(tmp_path, "claude", "Claude", "s", "mi clave es sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD")

    messages = load_state(tmp_path, "claude")["messages"]

    assert len(messages) == chat_state.MAX_MESSAGES and messages[0]["text"] == "mensaje 22"
    assert len(messages[-2]["text"]) == chat_state.MAX_MESSAGE_CHARS + 1 and messages[-1]["text"] == "(omitido: posible secreto)"
    assert "sk-" not in state_path(tmp_path, "claude").read_text(encoding="utf-8")


def test_a_new_chat_forgets_and_a_broken_file_is_ignored(tmp_path: Path) -> None:
    end_turn(tmp_path, "codex", "Codex", "t-1", ["hola"])
    reset_state(tmp_path, "codex")
    assert state_payload(tmp_path, "codex") == {**state_payload(tmp_path, "codex"), "session_id": "", "messages": []}

    state_path(tmp_path, "codex").write_text("{ no es json", encoding="utf-8")
    assert load_state(tmp_path, "codex") is None and state_payload(tmp_path, "codex")["messages"] == []
    begin_turn(tmp_path, "codex", "Codex", None, "sigue")        # and the next turn simply writes a good file
    assert load_state(tmp_path, "codex")["messages"][0]["text"] == "sigue"


def test_two_agents_of_a_project_keep_separate_conversations(tmp_path: Path) -> None:
    end_turn(tmp_path, "codex", "Codex", "a", ["de codex"])
    end_turn(tmp_path, "claude", "Claude", "b", ["de claude"])

    assert state_payload(tmp_path, "codex")["messages"][0]["text"] == "de codex"
    assert state_payload(tmp_path, "claude")["session_id"] == "b"


def test_restoring_marks_the_session_to_be_checked_once(tmp_path: Path, monkeypatch) -> None:
    end_turn(tmp_path, "codex", "Codex", "t-1", ["hola"])
    assert load_state(tmp_path, "codex")["verified"] is True

    state_payload(tmp_path, "codex", restoring=True)
    assert load_state(tmp_path, "codex")["verified"] is False

    monkeypatch.setattr(chat_resume, "session_exists", lambda *args: True)
    assert chat_resume.resolve_resume(tmp_path, "Codex", "codex", "t-1", "codex") == ("t-1", "")


def test_a_session_the_provider_lost_is_rebuilt_from_the_saved_conversation(tmp_path: Path, monkeypatch) -> None:
    begin_turn(tmp_path, "codex", "Codex", "t-1", "Quiero migrar el instalador")
    end_turn(tmp_path, "codex", "Codex", "t-1", ["Empecemos por Windows."])
    mark_unverified(tmp_path, "codex")
    monkeypatch.setattr(chat_resume, "session_exists", lambda *args: False)

    session, recap = chat_resume.resolve_resume(tmp_path, "Codex", "codex", "t-1", "codex")

    assert session is None
    assert "Persona: Quiero migrar el instalador" in recap and "Agente: Empecemos por Windows." in recap
    assert recap.index("Persona:") < recap.index("Agente:") and "<archivo-de-contexto" in recap


def test_nothing_is_rebuilt_when_the_session_is_known_or_cannot_be_told(tmp_path: Path, monkeypatch) -> None:
    end_turn(tmp_path, "codex", "Codex", "t-1", ["hola"])
    mark_unverified(tmp_path, "codex")
    for answer in (True, None):
        monkeypatch.setattr(chat_resume, "session_exists", lambda *args, answer=answer: answer)
        assert chat_resume.resolve_resume(tmp_path, "Codex", "codex", "t-1", "codex") == ("t-1", "")
    assert chat_resume.resolve_resume(tmp_path, "Codex", "codex", "otra", "codex") == ("otra", "")   # not the saved chat
    assert chat_resume.resolve_resume(tmp_path, "Codex", "codex", None, "codex") == (None, "")


def test_the_recap_is_bounded(tmp_path: Path) -> None:
    for index in range(60):
        end_turn(tmp_path, "codex", "Codex", "t", ["palabra " * 400 + str(index)])

    recap = recap_block(load_state(tmp_path, "codex"))

    assert len(recap) < 20_000 and recap.rstrip().endswith("</archivo-de-contexto>") and "59" in recap


def test_a_hand_off_is_written_whole_and_without_secrets(tmp_path: Path) -> None:
    path = save_handoff(tmp_path, "Codex", "t-1", "## Objetivo\nMigrar\ntoken = ghp_abcdefghijklmnopqrstuvwxyz0123456789\n## Pendiente\nTests")

    text = path.read_text(encoding="utf-8")
    assert "ghp_" not in text and "(línea omitida: posible secreto)" in text and "Migrar" in text and text.endswith("Tests\n")
    assert load_handoff(tmp_path, "Codex", "t-1").endswith("Tests")
    assert [p.name for p in path.parent.iterdir()] == [path.name]          # no temporary file left behind


def test_the_command_prints_the_contract_for_studio(tmp_path: Path) -> None:
    from typer.testing import CliRunner

    from chxchx_tech_lead.cli import app

    end_turn(tmp_path, "codex", "Codex", "t-1", ["hola"])
    runner = CliRunner()

    shown = json.loads(runner.invoke(app, ["agent", "chat-state", "codex", "--path", str(tmp_path), "--restore"]).stdout)
    assert shown["schema"] == "chxchx.chat-state" and shown["session_id"] == "t-1" and shown["messages"] == [{"role": "assistant", "text": "hola"}]
    assert load_state(tmp_path, "codex")["verified"] is False

    cleared = json.loads(runner.invoke(app, ["agent", "chat-state", "codex", "--path", str(tmp_path), "--reset"]).stdout)
    assert cleared["messages"] == [] and not state_path(tmp_path, "codex").exists()


def test_a_long_conversation_is_printed_as_one_line_of_valid_json(tmp_path: Path) -> None:
    from typer.testing import CliRunner

    from chxchx_tech_lead.cli import app

    end_turn(tmp_path, "codex", "Codex", "t-1", ["palabra " * 600, "otra línea\ncon salto"])

    out = CliRunner().invoke(app, ["agent", "chat-state", "codex", "--path", str(tmp_path)]).stdout

    assert out.count("\n") == 1 and json.loads(out)["messages"][0]["text"].startswith("palabra")
    assert json.loads(out)["messages"][1]["text"] == "otra línea\ncon salto"
