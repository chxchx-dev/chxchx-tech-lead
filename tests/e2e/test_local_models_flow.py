"""`local-models setup/remove/status` end to end: real CLI, real HTTP, real `ollama create` process."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.commands import local_models as command
from chxchx_tech_lead.integrations.local_hardware import Gpu, HardwareProfile
from chxchx_tech_lead.workspace import bridge_local_models
from fakes.fake_ollama import GIB

TAG = "qwen2.5-coder:7b"
VARIANT = f"{TAG}-ctx32k"
runner = CliRunner()


@pytest.fixture(autouse=True)
def machine(isolated_home, fake_ollama, fake_bin, monkeypatch) -> None:
    hardware = HardwareProfile(32 * GIB, 24 * GIB, 8, 500 * GIB, Path("/models"), (Gpu("RTX 4070", 12 * GIB, "nvidia"),))
    monkeypatch.setattr(command, "current_hardware", lambda: hardware)
    monkeypatch.setattr(bridge_local_models, "current_hardware", lambda: hardware)


def _run(project: Path, *args: str):
    result = runner.invoke(app, ["local-models", *args, "--path", str(project), "--json"])
    return result, [json.loads(line) for line in result.output.splitlines() if line.startswith("{")]


def _config(project: Path) -> dict:
    return json.loads((project / "opencode.json").read_text(encoding="utf-8"))


def test_setup_downloads_creates_the_variant_and_configures_opencode(tmp_path: Path, fake_ollama, fake_bin) -> None:
    result, events = _run(tmp_path, "setup", TAG, "--yes", "--set-default")

    assert result.exit_code == 0, result.output
    assert [e["step"] for e in events if e["type"] == "step"] == ["download", "context", "config"]
    assert any(e["type"] == "progress" and e["completed"] == e["total"] > 0 for e in events)
    assert events[-1]["changed"] is True and events[-1]["backup_path"] is None  # nothing existed to back up
    assert {TAG, VARIANT} <= set(fake_ollama.models)
    created = [call for call in fake_bin("ollama") if call["argv"][:1] == ["create"]]
    assert [call["argv"][1] for call in created] == [VARIANT]
    config = _config(tmp_path)
    assert config["model"] == f"ollama/{VARIANT}"
    assert config["provider"]["ollama"]["options"]["baseURL"] == f"http://{fake_ollama.host}/v1"
    assert config["provider"]["ollama"]["models"][VARIANT]["limit"]["context"] == 32768


def test_a_second_setup_changes_nothing(tmp_path: Path, fake_ollama, fake_bin) -> None:
    _run(tmp_path, "setup", TAG, "--yes")
    before = (tmp_path / "opencode.json").read_text(encoding="utf-8")

    result, events = _run(tmp_path, "setup", TAG, "--yes")

    assert result.exit_code == 0 and events[-1]["changed"] is False
    assert len(fake_ollama.calls("POST", "/api/pull")) == 1
    assert len([c for c in fake_bin("ollama") if c["argv"][:1] == ["create"]]) == 1
    assert (tmp_path / "opencode.json").read_text(encoding="utf-8") == before


def test_a_failed_download_leaves_the_project_untouched(tmp_path: Path, fake_ollama, fake_bin) -> None:
    fake_ollama.pull_error = "no space left on device"

    result, events = _run(tmp_path, "setup", TAG, "--yes")

    assert result.exit_code == 1 and events[-1]["type"] == "error"
    assert "no space left" in events[-1]["message"]
    assert not (tmp_path / "opencode.json").exists() and not fake_bin("ollama")


def test_a_failed_context_variant_does_not_write_the_config(tmp_path: Path, fake_ollama, monkeypatch) -> None:
    monkeypatch.setenv("FAKE_OLLAMA_CREATE_FAILS", "1")

    result, events = _run(tmp_path, "setup", TAG, "--yes")

    assert result.exit_code == 1 and "manifest" in events[-1]["message"]
    assert not (tmp_path / "opencode.json").exists()


def test_setup_without_confirmation_downloads_nothing(tmp_path: Path, fake_ollama) -> None:
    result, events = _run(tmp_path, "setup", TAG)

    assert result.exit_code == 1 and "--yes" in events[-1]["message"]
    assert fake_ollama.calls("POST", "/api/pull") == []


def test_dry_run_reports_the_plan_without_side_effects(tmp_path: Path, fake_ollama, fake_bin) -> None:
    result, events = _run(tmp_path, "setup", TAG, "--dry-run")

    assert result.exit_code == 0 and events[-1]["dry_run"] is True
    assert any(e["type"] == "preview" and VARIANT in e["content"] for e in events)
    assert fake_ollama.models == {} and not fake_bin("ollama") and not (tmp_path / "opencode.json").exists()


def test_status_payload_follows_the_lifecycle(tmp_path: Path, fake_ollama, monkeypatch) -> None:
    def row() -> dict:
        payload = json.loads(runner.invoke(app, ["bridge", "local-models", str(tmp_path)]).output)
        return next(model for model in payload["models"] if model["tag"] == TAG)

    assert (row()["downloaded"], row()["configured"], row()["loaded"]) == (False, False, False)
    _run(tmp_path, "setup", TAG, "--yes")
    fake_ollama.loaded = [{"name": VARIANT, "size": 5 * GIB, "size_vram": 5 * GIB}]
    assert (row()["downloaded"], row()["configured"], row()["loaded"]) == (True, True, True)


def test_remove_deletes_from_ollama_and_cleans_the_config(tmp_path: Path, fake_ollama) -> None:
    _run(tmp_path, "setup", TAG, "--yes", "--set-default")

    result, events = _run(tmp_path, "remove", TAG, "--yes")

    assert result.exit_code == 0 and events[-1]["removed"] == [VARIANT, TAG]
    assert fake_ollama.models == {}
    assert [body["model"] for body in fake_ollama.calls("DELETE", "/api/delete")] == [VARIANT, TAG]
    assert _config(tmp_path) == {"$schema": "https://opencode.ai/config.json"}
    assert events[-1]["backup_path"] and Path(events[-1]["backup_path"]).is_file()


def test_remove_keeps_the_config_when_ollama_refuses_to_delete(tmp_path: Path, fake_ollama) -> None:
    _run(tmp_path, "setup", TAG, "--yes", "--set-default")
    before = (tmp_path / "opencode.json").read_text(encoding="utf-8")
    fake_ollama.delete_status = 500

    result, events = _run(tmp_path, "remove", TAG, "--yes")

    assert result.exit_code == 1 and "HTTP 500" in events[-1]["message"]
    assert (tmp_path / "opencode.json").read_text(encoding="utf-8") == before


def test_user_settings_survive_setup_and_remove(tmp_path: Path, fake_ollama) -> None:
    (tmp_path / "opencode.json").write_text(json.dumps({"theme": "tokyonight", "mcp": {"serena": {"type": "local"}}}))

    _run(tmp_path, "setup", TAG, "--yes")
    _run(tmp_path, "remove", TAG, "--yes")

    config = _config(tmp_path)
    assert config["theme"] == "tokyonight" and config["mcp"] == {"serena": {"type": "local"}}
    assert "provider" not in config
