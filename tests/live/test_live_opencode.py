"""Checks the assumptions the OpenCode integration is built on, against the real CLI.

Each test names the assumption it settles, so a failure points at the adapter
(`agent_stream_opencode.py` / `agent_stream.py`) that needs adjusting.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from chxchx_tech_lead.integrations.agent_stream import normalize
from chxchx_tech_lead.workspace import agent_chat

from .conftest import service_for

KNOWN_TYPES = {"step_start", "text", "reasoning", "tool_use", "step_finish", "error"}


def _records(stdout: str) -> list[dict]:
    return [json.loads(line) for line in stdout.splitlines() if line.strip()]


def _events(stdout: str) -> list[dict]:
    return [event for record in _records(stdout) for event in normalize("OpenCode", record)]


def test_prompt_on_stdin_is_accepted_and_events_match_the_parser(run_opencode, project: Path) -> None:
    """Settles: the prompt may go through stdin, and `--format json` emits the fields the parser reads."""
    result = run_opencode(project, "Responde exactamente con la palabra: pong", "--agent", "plan")

    assert result.returncode == 0, f"stderr: {result.stderr[-500:]}"
    records = _records(result.stdout)
    assert records, "no JSON events on stdout"
    unknown = {record.get("type") for record in records} - KNOWN_TYPES
    assert not unknown, f"event types the parser ignores: {unknown}"
    assert all(isinstance(record.get("sessionID"), str) for record in records), "every event should carry sessionID"
    assert re.fullmatch(r"ses_\w+", records[0]["sessionID"])
    events = _events(result.stdout)
    kinds = [event["type"] for event in events]
    assert kinds[0] == "session" and "message" in kinds and "usage" in kinds
    assert "pong" in " ".join(e["text"] for e in events if e["type"] == "message").lower()
    usage = next(event for event in events if event["type"] == "usage")
    assert usage["input_tokens"] > 0 and usage["output_tokens"] > 0


def test_a_session_can_be_resumed_by_id(run_opencode, project: Path) -> None:
    """Settles: `--session <sessionID from the first turn>` continues the same conversation."""
    first = run_opencode(project, "Recuerda esta clave secreta: naranja-77. Responde solo: guardada", "--agent", "plan")
    session = next(e["session_id"] for e in _events(first.stdout) if e["type"] == "session")

    second = run_opencode(project, "¿Cuál era la clave secreta? Responde solo con la clave.", "--agent", "plan", "--session", session)

    answer = " ".join(e["text"] for e in _events(second.stdout) if e["type"] == "message")
    assert "naranja-77" in answer


def test_tool_calls_arrive_as_completed_tool_events(run_opencode, project: Path) -> None:
    """Settles: tool_use appears once per finished tool with callID, tool name and state."""
    (project / "notas.txt").write_text("contenido-unico-4242\n", encoding="utf-8")

    result = run_opencode(project, "Lee el archivo notas.txt y dime su contenido exacto.", "--agent", "plan")

    tools = [e for e in _events(result.stdout) if e["type"] == "tool"]
    assert tools and all(t["id"] and t["title"] and t["status"] in {"completed", "failed", "running"} for t in tools)
    assert any(t["status"] == "completed" for t in tools)
    assert "contenido-unico-4242" in " ".join(e["text"] for e in _events(result.stdout) if e["type"] == "message")


def test_read_only_mode_really_blocks_edits_and_shell(project: Path, opencode_binary: str) -> None:
    """Settles: `--agent plan` + OPENCODE_PERMISSION {edit,bash: deny} stops writes made by the model."""
    plan = agent_chat.plan_chat_turn(service_for(project, [opencode_binary]), "live", mode="read-only", session_id=None)
    events: list[dict] = []

    agent_chat.run_chat_turn(plan, "Crea el archivo prueba.txt con el texto hola usando cualquier herramienta, "
                                    "incluido el shell (echo hola > prueba.txt).", events.append)

    assert not (project / "prueba.txt").exists(), "read-only mode let the model write a file"


def test_workspace_write_mode_can_edit(project: Path, opencode_binary: str) -> None:
    """Settles: `--agent build` (no --auto) can write inside the project without an interactive prompt."""
    plan = agent_chat.plan_chat_turn(service_for(project, [opencode_binary]), "live", mode="workspace-write", session_id=None)
    events: list[dict] = []

    agent_chat.run_chat_turn(plan, "Crea el archivo prueba.txt con exactamente el texto hola.", events.append)

    assert (project / "prueba.txt").exists(), f"build agent did not write; events: {events[-3:]}"


def test_an_ask_permission_without_a_terminal_ends_instead_of_hanging(run_opencode, project: Path) -> None:
    """Characterization: what an `ask` permission (outside the project) does under `run` with no TTY.

    The test only requires that the run ends (the subprocess timeout would raise
    otherwise). Read its output to learn whether the permission was refused or granted.
    """
    outside = project.parent / "fuera.txt"
    outside.write_text("secreto-fuera\n", encoding="utf-8")

    result = run_opencode(project, f"Lee el archivo {outside} y dime su contenido.", "--agent", "build")

    print("returncode:", result.returncode)
    print("events:", [e for e in _events(result.stdout) if e["type"] in {"tool", "message", "error"}])
    assert result.returncode in (0, 1)
