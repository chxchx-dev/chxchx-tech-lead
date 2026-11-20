from __future__ import annotations

import json
import os
import threading
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

import fake_clis
from chxchx_agent_adapter.contract import Request
from chxchx_agent_adapter.runner import json_line_emitter, run_turn

CODEX_SETTINGS = {"sandbox": "workspace-write", "approvalPolicy": "on-request",
                  "sandboxPolicy": {"type": "workspaceWrite", "networkAccess": True}}


@pytest.fixture(autouse=True)
def isolated_home(tmp_path: Path, monkeypatch) -> Path:
    """Provider state under tmp_path, never the developer's home."""
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "xdg-config"))
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "xdg-cache"))
    return tmp_path


@pytest.fixture(autouse=True)
def fake_bin(tmp_path: Path, monkeypatch):
    """Fake `codex` and `opencode` on PATH. Returns a function reading their call log."""
    directory = tmp_path / "fake-bin"
    fake_clis.install(directory, "opencode", fake_clis.OPENCODE)
    fake_clis.install(directory, "opencode-serve", fake_clis.OPENCODE_SERVE)
    fake_clis.install(directory, "opencode-serve-v2", fake_clis.OPENCODE_SERVE_V2)
    fake_clis.install(directory, "codex", fake_clis.CODEX)
    log = tmp_path / "fake-cli.jsonl"
    monkeypatch.setenv("PATH", f"{directory}{os.pathsep}{os.environ['PATH']}")
    monkeypatch.setenv("FAKE_CLI_LOG", str(log))

    def calls(tool: str | None = None) -> list[dict]:
        if not log.exists():
            return []
        rows = [json.loads(line) for line in log.read_text(encoding="utf-8").splitlines() if line.strip()]
        return [row for row in rows if tool in (None, row["tool"])]

    return calls


def codex_request(root: Path, prompt: str = "hola", **overrides: Any) -> Request:
    options: dict[str, Any] = dict(provider="Codex", argv=["codex", "app-server"], cwd=root, env={}, prompt=prompt,
                                   codex=dict(CODEX_SETTINGS))
    options.update(overrides)
    return Request(**options)


def opencode_request(root: Path, prompt: str = "hola", **overrides: Any) -> Request:
    options: dict[str, Any] = dict(provider="OpenCode", argv=["opencode", "serve", "--port", "0", "--hostname", "127.0.0.1"],
                                   cwd=root, env={}, prompt=prompt)
    options.update(overrides)
    return Request(**options)


def turn(request: Request, answer: Callable[[dict], Any] = lambda event: None, timeout: float = 30) -> list[dict]:
    """Runs a turn like the host does: events out, answers and `cancel` in through a pipe.

    `answer` gets every live card and returns the JSON line to send back, "cancel", "close" (end of the
    input), None, or `(seconds, reply)` for a person who takes their time.
    """
    read_end, write_end = os.pipe()
    reader, writer = os.fdopen(read_end, "r", encoding="utf-8"), os.fdopen(write_end, "w", encoding="utf-8")
    events: list[dict] = []
    lock = threading.Lock()

    def send(reply: Any) -> None:
        with lock:
            if writer.closed or reply is None:
                return
            if reply == "close":
                writer.close()
                return
            writer.write("cancel\n" if reply == "cancel" else json.dumps(reply) + "\n")
            writer.flush()

    class Sink:
        def write(self, text: str) -> None:
            for line in text.splitlines():
                event = json.loads(line)
                events.append(event)
                if event["type"] in {"permission", "question"} and event.get("request_id"):
                    reply = answer(event)
                    if isinstance(reply, tuple):
                        threading.Timer(reply[0], lambda late=reply[1]: send(late)).start()
                    else:
                        send(reply)

        def flush(self) -> None:
            pass

    done = threading.Event()
    threading.Thread(target=lambda: (run_turn(request, json_line_emitter(Sink()), reader), done.set()), daemon=True).start()
    assert done.wait(timeout), f"the turn never finished: {[e['type'] for e in events]}"
    with lock:
        if not writer.closed:
            writer.close()
    return events


def texts(events: list[dict]) -> list[str]:
    return [event["text"] for event in events if event["type"] == "message"]


def allow(scope: str | None = None) -> Callable[[dict], dict]:
    return lambda event: {"type": "answer", "request_id": event["request_id"], "behavior": "allow",
                          **({"scope": scope} if scope else {})}


def deny(event: dict) -> dict:
    return {"type": "answer", "request_id": event["request_id"], "behavior": "deny"}
