"""The Ollama adapter against a real (fake) HTTP server: no stubs, real urllib."""

from __future__ import annotations

import pytest

from chxchx_tech_lead.integrations import ollama
from fakes.fake_ollama import GIB


def test_status_reads_version_models_and_loaded_over_http(fake_ollama, monkeypatch) -> None:
    monkeypatch.setattr(ollama, "find_executable", lambda name: "/usr/bin/ollama")
    fake_ollama.models = {"qwen3-coder:30b": 19 * GIB}
    fake_ollama.loaded = [{"name": "qwen3-coder:30b", "size": 22 * GIB, "size_vram": 20 * GIB}]

    status = ollama.ollama_status()

    assert (status.installed, status.running, status.version) == (True, True, "0.12.6")
    assert [(m.name, m.size_bytes) for m in status.models] == [("qwen3-coder:30b", 19 * GIB)]
    assert [(m.name, m.vram_bytes) for m in status.loaded] == [("qwen3-coder:30b", 20 * GIB)]


def test_status_when_nothing_listens(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_HOST", "127.0.0.1:1")  # a port nobody serves
    monkeypatch.setattr(ollama, "find_executable", lambda name: "/usr/bin/ollama")

    status = ollama.ollama_status()

    assert (status.installed, status.running, status.models, status.loaded) == (True, False, (), ())


def test_pull_streams_real_progress_and_registers_the_model(fake_ollama) -> None:
    fake_ollama.pull_size = 1000
    seen: list[tuple[str, int, int]] = []

    ollama.pull_model("qwen2.5-coder:7b", lambda *event: seen.append(event))

    assert seen[0] == ("pulling manifest", 0, 0) and seen[-1] == ("success", 0, 0)
    assert ("pulling 3f1a", 500, 1000) in seen and ("pulling 3f1a", 1000, 1000) in seen
    assert fake_ollama.calls("POST", "/api/pull") == [{"model": "qwen2.5-coder:7b", "stream": True}]
    assert "qwen2.5-coder:7b" in fake_ollama.models


def test_pull_error_event_becomes_an_ollama_error(fake_ollama) -> None:
    fake_ollama.pull_error = "pull model manifest: file does not exist"

    with pytest.raises(ollama.OllamaError, match="file does not exist"):
        ollama.pull_model("no-existe:1b", lambda *event: None)

    assert "no-existe:1b" not in fake_ollama.models


def test_cancelling_from_the_progress_callback_stops_the_download(fake_ollama) -> None:
    class Stop(Exception):
        pass

    def cancel(status: str, completed: int, total: int) -> None:
        if completed:
            raise Stop

    with pytest.raises(Stop):
        ollama.pull_model("qwen2.5-coder:7b", cancel)


def test_delete_removes_the_model_and_reports_http_errors(fake_ollama) -> None:
    fake_ollama.models = {"a:1": 1}

    ollama.delete_model("a:1")

    assert fake_ollama.models == {} and fake_ollama.calls("DELETE", "/api/delete") == [{"model": "a:1"}]
    with pytest.raises(ollama.OllamaError, match="HTTP 404"):
        ollama.delete_model("a:1")
    fake_ollama.delete_status = 500
    with pytest.raises(ollama.OllamaError, match="HTTP 500"):
        ollama.delete_model("a:1")


def test_unreachable_server_is_reported_not_raised_raw(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_HOST", "127.0.0.1:1")

    with pytest.raises(ollama.OllamaError, match="No pude hablar con Ollama"):
        ollama.pull_model("x:1", lambda *event: None)
    with pytest.raises(ollama.OllamaError, match="No pude hablar con Ollama"):
        ollama.delete_model("x:1")
