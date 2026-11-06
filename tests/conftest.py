from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from fakes import fake_clis
from fakes.fake_ollama import FakeOllama


@pytest.fixture
def isolated_home(tmp_path: Path, monkeypatch) -> Path:
    """ChxChx, Codex and Claude state under tmp_path, never the developer's home."""
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path / "claude"))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "xdg"))
    return tmp_path


@pytest.fixture
def fake_ollama(monkeypatch):
    """A running fake Ollama server; OLLAMA_HOST points the adapter at it."""
    server = FakeOllama().start()
    monkeypatch.setenv("OLLAMA_HOST", server.host)
    yield server
    server.stop()


@pytest.fixture
def fake_bin(tmp_path: Path, monkeypatch):
    """Fake `ollama` and `opencode` on PATH. Returns a function reading their call log."""
    directory = tmp_path / "fake-bin"
    fake_clis.install(directory, "ollama", fake_clis.OLLAMA)
    fake_clis.install(directory, "opencode", fake_clis.OPENCODE)
    fake_clis.install(directory, "claude", fake_clis.CLAUDE)
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
