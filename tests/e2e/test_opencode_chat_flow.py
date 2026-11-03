"""OpenCode in the visual chat end to end, with a real fake `opencode` process on PATH."""

from __future__ import annotations

import shutil
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.integrations.agent_catalog import agent_options, resolve_executable
from chxchx_tech_lead.workspace import agent_chat


def _service(root: Path, command: list[str] | None = None):
    agent = SimpleNamespace(id="opencode", command=command or ["opencode"], cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))


def _turn(root: Path, prompt: str = "¿Qué hace app.py?", **options):
    options.setdefault("mode", "read-only")
    options.setdefault("session_id", None)
    plan = agent_chat.plan_chat_turn(_service(root), "opencode", **options)
    events: list[dict] = []
    code = agent_chat.run_chat_turn(plan, prompt, events.append)
    return code, events


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_bin) -> None:
    pass


def test_new_chat_streams_normalized_events_and_resumes_by_session(tmp_path: Path, fake_bin) -> None:
    code, events = _turn(tmp_path)

    assert code == 0
    assert [e["type"] for e in events] == ["start", "session", "tool", "message", "usage", "limits", "done"]
    assert events[2] == {"type": "tool", "id": "c1", "kind": "command", "title": "bash: ls", "status": "completed", "output": "a.py\n"}
    assert events[3]["text"] == "eco: ¿Qué hace app.py?"
    assert events[4] == {"type": "usage", "input_tokens": 150, "output_tokens": 7, "cost_usd": 0.002}
    assert events[-1]["session_id"] == "ses_fake1"

    again, more = _turn(tmp_path, "sigue", session_id="ses_fake1", mode="workspace-write")
    run = [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]][-1]
    assert again == 0 and "--session" in run["argv"] and run["argv"][run["argv"].index("--session") + 1] == "ses_fake1"
    assert run["stdin"].strip() == "sigue" and "--agent" in run["argv"]


def test_the_launched_command_is_the_resolved_executable_path(tmp_path: Path, fake_bin, monkeypatch) -> None:
    """A bare `opencode` is launched by full path, so Windows `.cmd` shims work with CreateProcess."""
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "opencode", mode="read-only", session_id=None, images=(image,))
    resolved = shutil.which("opencode")

    assert plan.argv[0] == resolved and Path(resolved).is_absolute()
    events: list[dict] = []
    agent_chat.run_chat_turn(plan, "hola", events.append)  # rebuilds argv with staged images
    assert [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]][0]["argv"][-2] == "--file"
    assert resolve_executable("no-existe-xyz") == "no-existe-xyz" and resolve_executable("/x/opencode") == "/x/opencode"


def test_prompt_and_context_travel_by_stdin_not_argv(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, "pregunta única 12345")

    run = [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]][0]
    assert "pregunta única 12345" in run["stdin"] and "AGENTS.md" in run["stdin"]  # context preamble of a new chat
    assert not any("12345" in part for part in run["argv"])
    assert run["cwd"] == str(tmp_path.resolve())


def test_read_only_denies_edit_and_bash_and_write_mode_does_not(tmp_path: Path, fake_bin) -> None:
    _turn(tmp_path, mode="read-only")
    _turn(tmp_path, mode="workspace-write")

    ro, rw = [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]]
    assert ro["argv"][ro["argv"].index("--agent") + 1] == "plan" and ro["permission"] == '{"edit": "deny", "bash": "deny"}'
    assert rw["argv"][rw["argv"].index("--agent") + 1] == "build" and rw["permission"] is None


def test_model_effort_and_images_become_cli_arguments(tmp_path: Path, fake_bin) -> None:
    image = tmp_path / "captura.png"
    image.write_bytes(b"png")

    code, _ = _turn(tmp_path, model="ollama/qwen3-coder:30b-ctx32k", effort="high", images=(image,))

    run = [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]][0]["argv"]
    assert code == 0
    assert run[run.index("--model") + 1] == "ollama/qwen3-coder:30b-ctx32k" and run[run.index("--variant") + 1] == "high"
    staged = Path(run[run.index("--file") + 1])
    assert staged.parent == tmp_path.resolve() / ".ai" / "chat-attachments" and staged.read_bytes() == b"png"


def test_a_failing_turn_reports_the_agent_error_and_a_nonzero_exit(tmp_path: Path) -> None:
    code, events = _turn(tmp_path, "FALLA por favor")

    assert code == 1
    assert any(e["type"] == "error" and e["message"] == "sin cuota" for e in events)
    assert events[-1]["type"] == "done" and events[-1]["exit_code"] == 1


def test_the_session_is_summarized_into_the_journal_for_the_next_chat(tmp_path: Path) -> None:
    _turn(tmp_path)

    journal = (tmp_path / ".ai" / "history" / "SESSIONS.md").read_text(encoding="utf-8")
    assert "OpenCode" in journal and "ses_fake1" in journal and "Arregla la memoria por chat" in journal


def test_options_list_the_models_the_cli_reports(tmp_path: Path) -> None:
    options = agent_options("OpenCode", "opencode")

    assert options["available"] is True
    assert [m["id"] for m in options["models"]] == ["anthropic/claude-sonnet-5-5", "ollama/qwen3-coder:30b-ctx32k"]


def test_a_lean_turn_leaves_no_trace_in_opencode_or_in_the_project(tmp_path: Path, fake_bin) -> None:
    plan = agent_chat.plan_chat_turn(_service(tmp_path), "opencode", mode="workspace-write", session_id=None, lean=True)
    events: list[dict] = []

    code = agent_chat.run_chat_turn(plan, "Resume estos cambios", events.append)

    run = [call for call in fake_bin("opencode") if call["argv"][:1] == ["run"]][0]
    assert code == 0 and plan.mode == "read-only" and run["permission"] == '{"edit": "deny", "bash": "deny"}'
    assert run["stdin"].strip() == "Resume estos cambios"                      # no project context
    assert [call["argv"] for call in fake_bin("opencode") if call["argv"][:2] == ["session", "delete"]] == [["session", "delete", "ses_fake1"]]
    assert not (tmp_path / ".ai" / "history" / "SESSIONS.md").exists()          # no journal entry
    assert not [call for call in fake_bin("opencode") if call["argv"][:2] == ["session", "list"]]  # and no memory capture
