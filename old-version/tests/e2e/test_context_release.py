"""The context is freed automatically at 90%: a hand-off is written and the next message continues from it."""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import pytest
from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.workspace import agent_chat, chat_release


def _service(root: Path):
    agent = SimpleNamespace(id="opencode", command=["opencode"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin, tmp_path: Path, monkeypatch) -> None:
    cache = tmp_path / "cache" / "opencode"
    cache.mkdir(parents=True, exist_ok=True)
    (cache / "models.json").write_text(json.dumps({"x": {"models": {"y": {"limit": {"context": 1000}}}}}), encoding="utf-8")
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "config"))  # no default model of the person's own


def _turn(root: Path, prompt: str, tokens: int, *, session_id: str | None = None, model: str | None = "x/y", suffix: str = "",
          monkeypatch=None, **options):
    monkeypatch.setenv("FAKE_CONTEXT_TOKENS", str(tokens))
    monkeypatch.setenv("FAKE_SESSION_SUFFIX", suffix)
    plan = agent_chat.plan_chat_turn(_service(root), "opencode", mode="workspace-write", session_id=session_id, model=model, **options)
    events: list[dict] = []
    code = agent_chat.run_chat_turn(plan, prompt, events.append)
    return plan, code, events


def _runs(fake_bin) -> list[dict]:
    return [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]]


def _notices(events) -> list[str]:
    return [event["message"] for event in events if event["type"] == "notice"]


def _handoffs(root: Path) -> list[Path]:
    return sorted((root / ".ai" / "chat-handoffs").glob("*.md")) if (root / ".ai" / "chat-handoffs").exists() else []


def test_a_nearly_full_context_is_released_and_the_next_message_continues_from_the_hand_off(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _, code, events = _turn(tmp_path, "primera pregunta", 950, monkeypatch=monkeypatch)

    assert code == 0 and events[-1]["type"] == "done" and events[-1]["session_id"] == "ses_fake1"
    assert next(e for e in events if e["type"] == "limits")["context_window"] == 1000          # the window is completed for Studio
    assert [text for text in _notices(events) if "Contexto liberado" in text]                 # one discreet notice
    assert [e["text"] for e in events if e["type"] == "message"] == ["eco: primera pregunta"]    # the hand-off is not shown
    first, handoff_run = _runs(fake_bin)
    assert "--session" in handoff_run["argv"] and handoff_run["argv"][handoff_run["argv"].index("--session") + 1] == "ses_fake1"
    assert "liberar tu ventana de contexto" in handoff_run["stdin"] and handoff_run["permission"] == '{"edit": "deny", "bash": "deny"}'
    assert [p.read_text(encoding="utf-8") for p in _handoffs(tmp_path)] == ["## Objetivo\nTRASPASO: migrar el instalador\n"]

    # The next message of the same chat (Studio still names the old session) starts a clean one with the hand-off.
    plan, code, more = _turn(tmp_path, "segunda pregunta", 120, session_id="ses_fake1", suffix="b", monkeypatch=monkeypatch)
    resumed = _runs(fake_bin)[-1]
    assert code == 0 and plan.session_id is None and plan.continued_from == "ses_fake1"
    assert "--session" not in resumed["argv"]
    assert "TRASPASO: migrar el instalador" in resumed["stdin"] and "AGENTS.md" in resumed["stdin"] and resumed["stdin"].rstrip().endswith("segunda pregunta")
    assert more[-1]["session_id"] == "ses_fake1b" and not _handoffs(tmp_path)                    # consumed once the new session exists
    assert not any("Contexto liberado" in text for text in _notices(more))                             # no new release: the context is small


def test_below_the_threshold_nothing_happens(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _, _, events = _turn(tmp_path, "hola", 890, monkeypatch=monkeypatch)

    assert not _notices(events) and len(_runs(fake_bin)) == 1 and not _handoffs(tmp_path)


def test_a_model_whose_window_is_unknown_is_left_alone(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _, _, events = _turn(tmp_path, "hola", 999_999, model="sin/modelo", monkeypatch=monkeypatch)

    assert not _notices(events) and len(_runs(fake_bin)) == 1


def test_the_release_can_be_turned_off(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _, _, events = _turn(tmp_path, "hola", 990, auto_release=False, monkeypatch=monkeypatch)

    assert not _notices(events) and len(_runs(fake_bin)) == 1


def test_a_failed_turn_or_a_one_off_question_is_never_released(tmp_path: Path, fake_bin, monkeypatch) -> None:
    _, code, _ = _turn(tmp_path, "FALLA por favor", 990, monkeypatch=monkeypatch)
    _, _, lean = _turn(tmp_path, "pregunta suelta", 990, lean=True, monkeypatch=monkeypatch)

    assert code == 1 and not _notices(lean) and not _handoffs(tmp_path)


def test_a_failed_hand_off_keeps_the_conversation_going(tmp_path: Path, fake_bin, monkeypatch) -> None:
    monkeypatch.setattr(chat_release, "HANDOFF_PROMPT", "FALLA siempre: liberar tu ventana de contexto")

    _, code, events = _turn(tmp_path, "hola", 960, monkeypatch=monkeypatch)

    assert code == 0 and any("No pude liberar" in text for text in _notices(events)) and not _handoffs(tmp_path)
    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 0


def test_hidden_adapter_command_generates_a_read_only_handoff_event(
    tmp_path: Path, fake_bin, monkeypatch
) -> None:
    from chxchx_tech_lead.commands import agent_chat_handoff

    prompt = tmp_path / "handoff-prompt.txt"
    prompt.write_text("liberar tu ventana de contexto", encoding="utf-8")
    events: list[dict] = []
    monkeypatch.setattr(agent_chat_handoff, "workspace_service_for", lambda _path: _service(tmp_path))
    monkeypatch.setattr(
        agent_chat_handoff,
        "json_line_emitter",
        lambda: lambda event: events.append({"schema": "chxchx.agent-event", "schema_version": 1, **event}),
    )
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))
    cache = tmp_path / "cache" / "opencode"
    cache.mkdir(parents=True, exist_ok=True)
    (cache / "models.json").write_text("{}", encoding="utf-8")
    helper_help = CliRunner().invoke(app, ["agent", "chat-handoff", "--help"])
    assert helper_help.exit_code == 0 and "--session" in helper_help.stdout and "--prompt-file" in helper_help.stdout
    result = CliRunner().invoke(app, [
        "agent", "chat-handoff", "opencode", "--path", str(tmp_path),
        "--session", "ses_fake1", "--prompt-file", str(prompt), "--model", "x/y",
    ])

    payload = events[0]
    run = _runs(fake_bin)[-1]
    assert result.exit_code == 0, result.output
    assert len(events) == 1, "the helper emits only its final hidden result"
    assert payload["schema"] == "chxchx.agent-event" and payload["schema_version"] == 1
    assert payload["type"] == "handoff" and "TRASPASO" in payload["text"]
    assert "--session" in run["argv"] and run["argv"][run["argv"].index("--session") + 1] == "ses_fake1"
    assert run["permission"] == '{"edit": "deny", "bash": "deny"}'
    assert not (tmp_path / ".ai" / "chat-state").exists()
    assert not (tmp_path / ".ai" / "chat-handoffs").exists()
