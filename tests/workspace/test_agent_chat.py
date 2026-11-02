from __future__ import annotations

import io
import json
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.workspace import agent_chat
from chxchx_tech_lead.workspace.service import WorkspaceOperationError

FAKE_CODEX = r"""
import json, sys, time
# Like the real CLIs, speak UTF-8 regardless of the Windows console code page.
sys.stdout.reconfigure(encoding="utf-8")
prompt = sys.stdin.buffer.read().decode("utf-8")
print(json.dumps({"type": "thread.started", "thread_id": "t-fake"}), flush=True)
if "dormir" in prompt:
    time.sleep(30)
print(json.dumps({"type": "item.completed", "item": {"id": "i0", "type": "agent_message",
                  "text": "eco: " + prompt.strip().splitlines()[-1]}}), flush=True)
print("ruido que no es JSON", flush=True)
"""

FAKE_CLAUDE = r"""
import json, sys
sys.stdout.reconfigure(encoding="utf-8")
prompt = sys.stdin.buffer.read().decode("utf-8")
print(json.dumps({"type": "assistant", "message": {"content": [
    {"type": "text", "text": "eco: " + prompt.strip().splitlines()[-1]}]}}), flush=True)
"""


@pytest.fixture
def isolated_home(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path / "claude"))


def _service(root: Path, *, trusted: bool = True, command: list[str] | None = None):
    agent = SimpleNamespace(id="codex", command=command or ["codex"], cwd=".")
    inspection = SimpleNamespace(trusted=trusted, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


def _plan(root: Path, *, session_id: str | None = None) -> agent_chat.ChatTurnPlan:
    script = root / "fake_codex.py"
    script.write_text(FAKE_CODEX, encoding="utf-8")
    return agent_chat.ChatTurnPlan("codex", "Codex", root, root, [sys.executable, str(script)], "read-only", session_id)


def test_plan_requires_trust_and_a_supported_agent(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(agent_chat, "executable_available", lambda executable: True)
    with pytest.raises(WorkspaceOperationError, match="confiable"):
        agent_chat.plan_chat_turn(_service(tmp_path, trusted=False), "codex", mode="read-only", session_id=None)
    with pytest.raises(WorkspaceOperationError, match="Codex y Claude"):
        agent_chat.plan_chat_turn(_service(tmp_path, command=["opencode"]), "codex", mode="read-only", session_id=None)
    with pytest.raises(WorkspaceOperationError, match="No existe"):
        agent_chat.plan_chat_turn(_service(tmp_path), "claude", mode="read-only", session_id=None)

    plan = agent_chat.plan_chat_turn(_service(tmp_path), "codex", mode="workspace-write", session_id="t1")
    assert plan.provider == "Codex" and plan.argv[-3:] == ["resume", "t1", "-"]


def test_new_chat_turn_streams_events_with_context(tmp_path: Path, isolated_home) -> None:
    events: list[dict] = []

    exit_code = agent_chat.run_chat_turn(_plan(tmp_path), "¿Qué imprime app.py?", events.append)

    assert exit_code == 0
    assert [event["type"] for event in events] == ["start", "session", "message", "done"]
    assert events[2]["text"] == "eco: ¿Qué imprime app.py?"
    assert events[-1] == {"type": "done", "exit_code": 0, "cancelled": False, "session_id": "t-fake"}


def test_cancel_line_stops_the_agent(tmp_path: Path, isolated_home) -> None:
    events: list[dict] = []
    started = time.monotonic()

    agent_chat.run_chat_turn(_plan(tmp_path, session_id="t-fake"), "dormir", events.append,
                             cancel_input=io.StringIO("cancel\n"))

    assert time.monotonic() - started < 20
    assert events[-1]["cancelled"] is True
    assert not any(event["type"] == "error" for event in events)


def test_missing_executable_reports_error_and_done(tmp_path: Path, isolated_home) -> None:
    plan = agent_chat.ChatTurnPlan("codex", "Codex", tmp_path, tmp_path, [str(tmp_path / "no-existe")], "read-only", "t")
    events: list[dict] = []

    assert agent_chat.run_chat_turn(plan, "hola", events.append) == 127
    assert [event["type"] for event in events] == ["start", "error", "done"]


def test_emitter_adds_versioned_schema() -> None:
    stream = io.StringIO()
    agent_chat.json_line_emitter(stream)({"type": "done", "exit_code": 0})
    assert json.loads(stream.getvalue()) == {
        "schema": "chxchx.agent-event", "schema_version": 1, "type": "done", "exit_code": 0
    }


def test_plan_explains_how_to_install_a_missing_agent(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(agent_chat, "executable_available", lambda executable: False)

    with pytest.raises(WorkspaceOperationError, match="install.sh"):
        agent_chat.plan_chat_turn(_service(tmp_path, command=["claude"]), "codex", mode="read-only", session_id=None)


def test_plan_validates_images_and_passes_model_and_effort(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(agent_chat, "executable_available", lambda executable: True)
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")

    plan = agent_chat.plan_chat_turn(_service(tmp_path), "codex", mode="read-only", session_id=None,
                                     model="gpt-x", effort="high", images=(image,))
    assert ["-m", "gpt-x"] == plan.argv[plan.argv.index("-m"):plan.argv.index("-m") + 2]
    assert str(image) in plan.argv

    with pytest.raises(WorkspaceOperationError, match="no soportado"):
        agent_chat.plan_chat_turn(_service(tmp_path), "codex", mode="read-only", session_id=None,
                                  images=(tmp_path / "notas.txt",))


def test_claude_images_are_staged_in_the_project_and_listed_in_the_prompt(tmp_path: Path, isolated_home) -> None:
    image = tmp_path / "pantalla.png"
    image.write_bytes(b"png")
    script = tmp_path / "fake_claude.py"
    script.write_text(FAKE_CLAUDE, encoding="utf-8")
    plan = agent_chat.ChatTurnPlan("claude", "Claude", tmp_path, tmp_path, [sys.executable, str(script)],
                                   "read-only", "s-fake", ("claude",), None, None, (image,))
    events: list[dict] = []

    agent_chat.run_chat_turn(plan, "¿Qué ves?", events.append)

    staged = list((tmp_path / ".ai" / "chat-attachments").iterdir())
    assert len(staged) == 1 and staged[0].read_bytes() == b"png"
    assert events[1]["text"].startswith("eco: - .ai/chat-attachments/")
