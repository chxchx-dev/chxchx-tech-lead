from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.integrations import ollama
from chxchx_tech_lead.integrations.local_hardware import HardwareProfile
from chxchx_tech_lead.commands import local_models as command
from chxchx_tech_lead.workspace import bridge_local_models

GIB = 1024**3
runner = CliRunner()


def _patch(monkeypatch, *, installed: bool = True) -> None:
    hardware = HardwareProfile(32 * GIB, 24 * GIB, 8, 500 * GIB, Path("/models"))
    monkeypatch.setattr(bridge_local_models, "current_hardware", lambda: hardware)
    monkeypatch.setattr(command, "current_hardware", lambda: hardware)
    status = ollama.OllamaStatus(installed, installed, "http://127.0.0.1:11434", "0.12.6" if installed else None,
                                 (ollama.InstalledModel("qwen2.5-coder:7b", 1),) if installed else (),
                                 (ollama.LoadedModel("qwen2.5-coder:7b-ctx32k", 5 * GIB, 4 * GIB),) if installed else ())
    monkeypatch.setattr(bridge_local_models, "ollama_status", lambda: status)
    monkeypatch.setattr(bridge_local_models, "executable_available", lambda name: False)


def test_bridge_payload_is_versioned_and_lists_fit_per_model(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch)

    result = runner.invoke(app, ["bridge", "local-models", str(tmp_path)])

    payload = json.loads(result.output)
    assert result.exit_code == 0 and payload["schema"] == "chxchx.local-models"
    assert payload["ollama"]["running"] is True and payload["opencode"]["installed"] is False
    assert payload["hardware"]["accelerator"] == "sin GPU compatible"
    assert {model["fit"] for model in payload["models"]} <= {"gpu", "partial", "cpu", "none"}
    assert sum(model["recommended"] for model in payload["models"]) <= 1


def test_status_tells_how_to_install_missing_tools(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch, installed=False)

    result = runner.invoke(app, ["local-models", "status", "--path", str(tmp_path)])

    assert result.exit_code == 0
    assert "Ollama no está instalado" in result.output and "npm install -g opencode-ai" in result.output


def test_setup_dry_run_json_emits_events_and_writes_nothing(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch)
    from chxchx_tech_lead.integrations import local_models_setup

    monkeypatch.setattr(local_models_setup, "ollama_status", lambda: ollama.OllamaStatus(True, True, "http://127.0.0.1:11434"))

    result = runner.invoke(app, ["local-models", "setup", "qwen2.5-coder:7b", "--path", str(tmp_path), "--dry-run", "--json"])

    events = [json.loads(line) for line in result.output.splitlines()]
    assert result.exit_code == 0 and all(e["schema"] == "chxchx.local-models-event" for e in events)
    assert [e["type"] for e in events][-2:] == ["preview", "done"] and events[-1]["dry_run"] is True
    assert not (tmp_path / "opencode.json").exists()


def test_setup_failure_exits_non_zero_with_a_json_error(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch)

    result = runner.invoke(app, ["local-models", "setup", "no-existe", "--path", str(tmp_path), "--json"])

    assert result.exit_code == 1 and json.loads(result.output.splitlines()[-1])["type"] == "error"


def test_bridge_payload_reports_downloaded_and_loaded_models(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch)

    payload = json.loads(runner.invoke(app, ["bridge", "local-models", str(tmp_path)]).output)

    row = next(model for model in payload["models"] if model["tag"] == "qwen2.5-coder:7b")
    assert row["downloaded"] is True and row["loaded"] is True
    assert payload["ollama"]["loaded"] == [{"name": "qwen2.5-coder:7b-ctx32k", "size_bytes": 5 * GIB, "vram_bytes": 4 * GIB}]
    assert not any(model["loaded"] for model in payload["models"] if model["tag"] != "qwen2.5-coder:7b")


def test_remove_json_streams_events_and_reports_errors(tmp_path: Path, monkeypatch) -> None:
    _patch(monkeypatch)
    from chxchx_tech_lead.integrations import local_models_remove

    deleted: list[str] = []
    status = ollama.OllamaStatus(True, True, "http://127.0.0.1:11434", "1", (ollama.InstalledModel("qwen2.5-coder:7b", 1),))
    monkeypatch.setattr(local_models_remove, "ollama_status", lambda: status)
    monkeypatch.setattr(local_models_remove, "delete_model", deleted.append)

    result = runner.invoke(app, ["local-models", "remove", "qwen2.5-coder:7b", "--path", str(tmp_path), "--yes", "--json"])

    events = [json.loads(line) for line in result.output.splitlines()]
    assert result.exit_code == 0 and deleted == ["qwen2.5-coder:7b"]
    assert events[-1]["type"] == "done" and events[-1]["removed"] == ["qwen2.5-coder:7b"]

    refused = runner.invoke(app, ["local-models", "remove", "qwen2.5-coder:7b", "--path", str(tmp_path), "--json"])
    assert refused.exit_code == 1 and "--yes" in json.loads(refused.output.splitlines()[-1])["message"]
