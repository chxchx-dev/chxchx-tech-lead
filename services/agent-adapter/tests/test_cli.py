"""The command line: what a host sees when it runs the service as a process."""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
from pathlib import Path

from conftest import codex_request

from chxchx_agent_adapter import VERSION
from chxchx_agent_adapter.cli import main


def test_the_contract_command_describes_the_service() -> None:
    out = io.StringIO()

    assert main(["contract"], io.StringIO(), out) == 0
    assert json.loads(out.getvalue()) == {"schema": "chxchx.agent-adapter", "contract_version": 1, "version": VERSION,
                                          "providers": ["Codex", "OpenCode"]}


def test_a_request_that_breaks_the_contract_exits_with_two_and_says_why() -> None:
    out = io.StringIO()

    assert main(["run"], io.StringIO('{"schema":"x"}\n'), out) == 2
    events = [json.loads(line) for line in out.getvalue().splitlines()]
    assert [e["type"] for e in events] == ["error", "adapter_done"] and events[-1]["exit_code"] == 2


def test_unknown_commands_are_a_usage_error() -> None:
    assert main(["fly"], io.StringIO(), io.StringIO()) == 2


def test_run_as_a_real_process_end_to_end(tmp_path: Path, fake_bin) -> None:
    request = codex_request(tmp_path, "hola")
    line = json.dumps({"schema": "chxchx.agent-adapter-request", "schema_version": 1, "provider": "Codex",
                       "argv": request.argv, "cwd": str(tmp_path), "env": {}, "prompt": "hola", "codex": request.codex})
    env = {**os.environ, "PYTHONPATH": str(Path(__file__).parents[1] / "src")}

    process = subprocess.Popen([sys.executable, "-m", "chxchx_agent_adapter", "run"], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
    assert process.stdin is not None and process.stdout is not None
    process.stdin.write(line + "\n")  # the input stays open: its end means the host went away
    process.stdin.flush()
    events = []
    for row in process.stdout:
        events.append(json.loads(row))
        if events[-1]["type"] == "adapter_done":
            break
    process.stdin.close()
    assert process.wait(timeout=30) == 0, process.stderr.read() if process.stderr else ""

    assert [e["type"] for e in events] == ["session", "message", "usage", "limits", "adapter_done"]
    assert events[-1]["exit_code"] == 0 and events[1]["text"] == "eco: hola"


def test_the_end_of_the_input_stops_the_turn_and_its_provider(tmp_path: Path, fake_bin) -> None:
    request = codex_request(tmp_path, "PERMISO")
    line = json.dumps({"schema": "chxchx.agent-adapter-request", "schema_version": 1, "provider": "Codex",
                       "argv": request.argv, "cwd": str(tmp_path), "env": {}, "prompt": "PERMISO", "codex": request.codex})
    env = {**os.environ, "PYTHONPATH": str(Path(__file__).parents[1] / "src")}

    process = subprocess.Popen([sys.executable, "-m", "chxchx_agent_adapter", "run"], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, text=True, env=env)
    assert process.stdin is not None and process.stdout is not None
    process.stdin.write(line + "\n")
    process.stdin.flush()
    for row in process.stdout:
        if json.loads(row)["type"] == "permission":
            break  # the provider now waits for the person
    process.stdin.close()  # the host went away without answering

    events = [json.loads(row) for row in process.stdout]
    assert process.wait(timeout=30) == 0
    assert events[-1]["type"] == "adapter_done" and events[-1]["cancelled"] is True
