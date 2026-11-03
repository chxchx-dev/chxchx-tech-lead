from __future__ import annotations

import io
import json
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.integrations import local_hardware, local_models_remove, local_models_setup, ollama, opencode_local_config
from chxchx_tech_lead.integrations.local_hardware import Gpu, HardwareProfile, detect_hardware
from chxchx_tech_lead.integrations.local_model_catalog import CATALOG, GIB, find_model
from chxchx_tech_lead.integrations.local_model_fit import assess, recommend
from chxchx_tech_lead.integrations.local_models_remove import remove_local_model
from chxchx_tech_lead.integrations.local_models_setup import LocalModelSetupError, setup_local_model
from chxchx_tech_lead.integrations.opencode_local_config import (
    OpenCodeConfigError, apply_change, plan_ollama_model, plan_remove_ollama_models)


def _hardware(*, vram_gb: float = 0, ram_gb: float = 16, free_ram_gb: float | None = None, disk_gb: float = 500,
              unified: bool = False) -> HardwareProfile:
    gpus = (Gpu("GPU", int(vram_gb * GIB), "nvidia"),) if vram_gb else ()
    return HardwareProfile(int(ram_gb * GIB), int((free_ram_gb if free_ram_gb is not None else ram_gb) * GIB), 8,
                           int(disk_gb * GIB), Path("/models"), gpus, unified)


# --- hardware ---------------------------------------------------------------

def test_reads_nvidia_memory_from_nvidia_smi(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setattr(local_hardware.shutil, "which", lambda name: "/usr/bin/nvidia-smi")
    output = "NVIDIA GeForce RTX 4070, 12282\nNVIDIA GeForce RTX 3060, 12288\n"
    run = lambda *args, **kwargs: subprocess.CompletedProcess(args, 0, output, "")

    hardware = detect_hardware(32 * GIB, 20 * GIB, models_dir=tmp_path, run=run)

    assert [gpu.vram_bytes // 1024**2 for gpu in hardware.gpus if gpu.vendor == "nvidia"] == [12282, 12288]
    assert hardware.accelerated_bytes == 12288 * 1024**2


def test_survives_missing_or_failing_nvidia_smi(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setattr(local_hardware.shutil, "which", lambda name: None)
    assert local_hardware._nvidia_gpus(subprocess.run) == ()
    monkeypatch.setattr(local_hardware.shutil, "which", lambda name: "/usr/bin/nvidia-smi")

    def broken(*args, **kwargs):
        raise subprocess.TimeoutExpired(args, 5)

    assert local_hardware._nvidia_gpus(broken) == ()
    assert local_hardware._nvidia_gpus(lambda *a, **k: subprocess.CompletedProcess(a, 9, "", "")) == ()


def test_reads_amd_vram_from_sysfs_and_ignores_other_vendors(tmp_path: Path) -> None:
    for index, (vendor, vram) in enumerate((("0x1002", "8589934592"), ("0x8086", "1"))):
        device = tmp_path / f"card{index}" / "device"
        device.mkdir(parents=True)
        (device / "vendor").write_text(vendor + "\n")
        (device / "mem_info_vram_total").write_text(vram + "\n")

    gpus = local_hardware._amd_gpus(tmp_path)

    assert [(gpu.vendor, gpu.vram_bytes) for gpu in gpus] == [("amd", 8589934592)]


def test_models_dir_prefers_the_environment_override(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setenv("OLLAMA_MODELS", str(tmp_path / "m"))
    assert local_hardware.default_models_dir() == tmp_path / "m"
    assert detect_hardware(8 * GIB, 4 * GIB, models_dir=tmp_path / "m" / "nested", run=subprocess.run).disk_free_bytes > 0


def test_unified_memory_counts_two_thirds_of_ram_as_accelerated() -> None:
    assert _hardware(ram_gb=32, unified=True).accelerated_bytes == int(32 * GIB * 0.66)


# --- fit --------------------------------------------------------------------

def test_catalog_only_lists_models_the_library_marks_with_tools() -> None:
    assert find_model("qwen3-coder:30b").size_gb == 19.0
    assert find_model("nope") is None
    assert len({model.tag for model in CATALOG}) == len(CATALOG)


@pytest.mark.parametrize(("hardware", "tag", "fit"), [
    (_hardware(vram_gb=24), "qwen3-coder:30b", "gpu"),
    (_hardware(vram_gb=16, free_ram_gb=16), "qwen3-coder:30b", "partial"),
    (_hardware(vram_gb=8, free_ram_gb=16), "qwen3-coder:30b", "none"),
    (_hardware(free_ram_gb=16), "qwen2.5-coder:7b", "cpu"),
    (_hardware(free_ram_gb=16), "qwen3-coder:30b", "none"),
    (_hardware(vram_gb=8, free_ram_gb=1), "gpt-oss:120b", "none"),
    (_hardware(ram_gb=64, unified=True), "qwen3-coder:30b", "gpu"),
    (_hardware(ram_gb=64, free_ram_gb=4, unified=True), "qwen3-coder:30b", "none"),
])
def test_assess_places_models_by_memory(hardware: HardwareProfile, tag: str, fit: str) -> None:
    assert assess(find_model(tag), hardware).fit == fit


def test_cpu_only_never_offers_models_too_large_to_be_usable() -> None:
    assert assess(find_model("qwen2.5-coder:14b"), _hardware(ram_gb=64)).fit == "cpu"
    assert assess(find_model("qwen2.5-coder:32b"), _hardware(ram_gb=128)).fit == "none"


def test_recommend_puts_the_best_fitting_model_first_and_respects_disk() -> None:
    ranked = recommend(_hardware(vram_gb=24))
    assert ranked[0].model.tag == "qwen3-coder:30b" and ranked[0].fit == "gpu" and len(ranked) == len(CATALOG)
    small_disk = recommend(_hardware(vram_gb=24, disk_gb=10))
    assert small_disk[0].model.size_gb <= 9 and small_disk[0].disk_ok
    assert all(item.fit == "none" for item in recommend(_hardware(ram_gb=2)))


# --- ollama adapter ---------------------------------------------------------

class _Response(io.BytesIO):
    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


def test_base_url_normalizes_ollama_host(monkeypatch) -> None:
    for raw, expected in (("", "http://127.0.0.1:11434"), ("0.0.0.0:9999", "http://127.0.0.1:9999"),
                          (":8080", "http://127.0.0.1:8080"), ("https://gpu-box:11435/", "https://gpu-box:11435"),
                          ("gpu-box", "http://gpu-box:11434")):
        monkeypatch.setenv("OLLAMA_HOST", raw)
        assert ollama.base_url() == expected


def test_status_reports_models_or_a_stopped_server(monkeypatch) -> None:
    monkeypatch.delenv("OLLAMA_HOST", raising=False)
    monkeypatch.setattr(ollama, "find_executable", lambda name: "/usr/bin/ollama")
    answers = {"/api/version": {"version": "0.12.6"},
               "/api/tags": {"models": [{"name": "qwen3-coder:30b", "size": 19 * GIB}]},
               "/api/ps": {"models": [{"name": "qwen3-coder:30b-ctx32k", "size": 22 * GIB, "size_vram": 20 * GIB}]}}
    opener = lambda url, timeout: _Response(json.dumps(answers[url.rsplit(":11434", 1)[1]]).encode())

    status = ollama.ollama_status(opener=opener)

    assert (status.installed, status.running, status.version) == (True, True, "0.12.6")
    assert status.models[0].name == "qwen3-coder:30b"
    assert [(item.name, item.vram_bytes) for item in status.loaded] == [("qwen3-coder:30b-ctx32k", 20 * GIB)]

    def refuse(url, timeout):
        raise OSError("connection refused")

    stopped = ollama.ollama_status(opener=refuse)
    assert (stopped.installed, stopped.running, stopped.models) == (True, False, ())


def test_pull_streams_progress_and_surfaces_errors() -> None:
    lines = [{"status": "pulling abc", "total": 10, "completed": 4}, {"status": "success"}]
    seen: list[tuple] = []
    ollama.pull_model("m", lambda *args: seen.append(args),
                      opener=lambda request, timeout: _Response("\n".join(map(json.dumps, lines)).encode()))
    assert seen == [("pulling abc", 4, 10), ("success", 0, 0)]

    with pytest.raises(ollama.OllamaError, match="no space"):
        ollama.pull_model("m", lambda *args: None,
                          opener=lambda request, timeout: _Response(b'{"error": "no space left"}\n'))


def test_context_variant_is_created_with_the_official_cli() -> None:
    calls: list[list[str]] = []

    def run(command, **kwargs):
        calls.append(command)
        assert Path(command[-1]).read_text(encoding="utf-8") == "FROM qwen3-coder:30b\nPARAMETER num_ctx 32768\n"
        return subprocess.CompletedProcess(command, 0, "", "")

    assert ollama.create_context_variant("qwen3-coder:30b", 32768, run=run) == "qwen3-coder:30b-ctx32k"
    assert calls[0][:3] == ["ollama", "create", "qwen3-coder:30b-ctx32k"]
    with pytest.raises(ollama.OllamaError, match="boom"):
        ollama.create_context_variant("m", 8192, run=lambda command, **kw: subprocess.CompletedProcess(command, 1, "", "boom"))


def test_install_instructions_never_run_anything() -> None:
    assert "install.sh" in ollama.install_instructions("linux")["command"]
    assert ollama.install_instructions("win32")["command"] is None


# --- opencode.json ----------------------------------------------------------

def _plan(root: Path, **overrides):
    options = {"label": "Qwen3", "context": 32768, "endpoint": "http://127.0.0.1:11434", "set_default": False} | overrides
    return plan_ollama_model(root, "qwen3-coder:30b-ctx32k", **options)


def test_config_merge_keeps_user_keys_and_is_idempotent(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    (tmp_path / "opencode.json").write_text(json.dumps({
        "theme": "tokyonight", "model": "anthropic/claude-sonnet-5-5",
        "provider": {"ollama": {"options": {"baseURL": "http://gpu-box:11434/v1"}, "models": {"llama3": {"name": "L"}}}}}))

    change = _plan(tmp_path)
    backup = apply_change(tmp_path, change)
    data = json.loads((tmp_path / "opencode.json").read_text())

    assert data["theme"] == "tokyonight" and data["model"] == "anthropic/claude-sonnet-5-5"
    assert data["provider"]["ollama"]["options"]["baseURL"] == "http://gpu-box:11434/v1"
    assert set(data["provider"]["ollama"]["models"]) == {"llama3", "qwen3-coder:30b-ctx32k"}
    assert data["provider"]["ollama"]["models"]["qwen3-coder:30b-ctx32k"]["limit"] == {"context": 32768, "output": 8192}
    assert backup is not None and json.loads(backup.read_text())["theme"] == "tokyonight"
    assert not _plan(tmp_path).changed and apply_change(tmp_path, _plan(tmp_path)) is None
    assert opencode_local_config.configured_ollama_models(tmp_path) == ["llama3", "qwen3-coder:30b-ctx32k"]


def test_config_for_a_new_project_and_default_model(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    change = _plan(tmp_path, set_default=True)
    assert apply_change(tmp_path, change) is None  # nothing to back up
    data = json.loads((tmp_path / "opencode.json").read_text())
    assert data["model"] == "ollama/qwen3-coder:30b-ctx32k" == change.model_ref
    assert data["provider"]["ollama"]["npm"] == "@ai-sdk/openai-compatible"
    assert data["provider"]["ollama"]["options"]["baseURL"] == "http://127.0.0.1:11434/v1"
    assert not list(tmp_path.glob(".opencode-*.tmp"))


def test_config_refuses_files_it_cannot_merge_without_losing_data(tmp_path: Path) -> None:
    (tmp_path / "opencode.json").write_text("{ no es json")
    with pytest.raises(OpenCodeConfigError, match="JSON válido"):
        _plan(tmp_path)
    (tmp_path / "opencode.json").write_text('{"provider": {"ollama": []}}')
    with pytest.raises(OpenCodeConfigError, match="objeto"):
        _plan(tmp_path)
    (tmp_path / "opencode.json").unlink()
    (tmp_path / "opencode.jsonc").write_text("{}")
    with pytest.raises(OpenCodeConfigError, match="jsonc"):
        _plan(tmp_path)


def test_backup_area_does_not_interfere_with_project_rollback(tmp_path: Path, monkeypatch) -> None:
    from chxchx_tech_lead.core.backup import latest_backup

    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    (tmp_path / "opencode.json").write_text("{}")
    apply_change(tmp_path, _plan(tmp_path))
    assert latest_backup(tmp_path) is None


# --- use case ---------------------------------------------------------------

def _stub_ollama(monkeypatch, *, installed=True, running=True, models=()):
    status = ollama.OllamaStatus(installed, running, "http://127.0.0.1:11434", "0.12.6",
                                 tuple(ollama.InstalledModel(name, 1) for name in models))
    monkeypatch.setattr(local_models_setup, "ollama_status", lambda: status)
    calls: list = []
    monkeypatch.setattr(local_models_setup, "pull_model", lambda tag, report: (calls.append(("pull", tag)), report("success", 1, 1)))
    monkeypatch.setattr(local_models_setup, "create_context_variant", lambda tag, ctx: calls.append(("create", tag, ctx)))
    return calls


def test_setup_requires_confirmation_before_downloading(tmp_path: Path, monkeypatch) -> None:
    calls = _stub_ollama(monkeypatch)
    with pytest.raises(LocalModelSetupError, match="--yes"):
        setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), lambda event: None)
    assert calls == [] and not (tmp_path / "opencode.json").exists()


def test_setup_downloads_creates_the_variant_and_writes_config(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    calls = _stub_ollama(monkeypatch)
    events: list[dict] = []

    result = setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), events.append, confirmed=True, set_default=True)

    assert calls == [("pull", "qwen2.5-coder:7b"), ("create", "qwen2.5-coder:7b", 32768)]
    assert result.model_ref == "ollama/qwen2.5-coder:7b-ctx32k" and result.changed
    assert [e["type"] for e in events] == ["step", "progress", "step", "step"]
    assert json.loads((tmp_path / "opencode.json").read_text())["model"] == result.model_ref


def test_setup_skips_steps_that_are_already_done(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    calls = _stub_ollama(monkeypatch, models=("qwen2.5-coder:7b", "qwen2.5-coder:7b-ctx32k"))
    setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), lambda event: None)
    assert calls == []


def test_dry_run_changes_nothing_and_previews_the_config(tmp_path: Path, monkeypatch) -> None:
    calls = _stub_ollama(monkeypatch, running=False)
    events: list[dict] = []

    result = setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), events.append, dry_run=True)

    assert calls == [] and not (tmp_path / "opencode.json").exists() and result.backup_path is None
    assert any(e["type"] == "preview" and "qwen2.5-coder:7b-ctx32k" in e["content"] for e in events)


def test_setup_explains_why_it_cannot_continue(tmp_path: Path, monkeypatch) -> None:
    _stub_ollama(monkeypatch, installed=False)
    with pytest.raises(LocalModelSetupError, match="no está instalado"):
        setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), lambda event: None)
    _stub_ollama(monkeypatch, running=False)
    with pytest.raises(LocalModelSetupError, match="no responde"):
        setup_local_model(tmp_path, "qwen2.5-coder:7b", _hardware(vram_gb=12), lambda event: None)
    _stub_ollama(monkeypatch)
    with pytest.raises(LocalModelSetupError, match="no cabe"):
        setup_local_model(tmp_path, "gpt-oss:120b", _hardware(ram_gb=8), lambda event: None)
    with pytest.raises(LocalModelSetupError, match="catálogo"):
        setup_local_model(tmp_path, "desconocido", _hardware(vram_gb=12), lambda event: None)


# --- loaded models and removal ----------------------------------------------

def test_status_survives_a_failing_ps_endpoint(monkeypatch) -> None:
    monkeypatch.setattr(ollama, "find_executable", lambda name: "/usr/bin/ollama")

    def opener(url, timeout):
        if url.endswith("/api/ps"):
            raise OSError("boom")
        return _Response(json.dumps({"version": "1", "models": []}).encode())

    status = ollama.ollama_status(opener=opener)

    assert status.running and status.loaded == ()


def test_delete_uses_the_documented_endpoint_and_reports_failures() -> None:
    seen: list = []

    def opener(request, timeout):
        seen.append((request.get_method(), request.full_url, json.loads(request.data)))
        return _Response(b"")

    ollama.delete_model("qwen3-coder:30b", opener=opener)
    assert seen == [("DELETE", "http://127.0.0.1:11434/api/delete", {"model": "qwen3-coder:30b"})]

    def not_found(request, timeout):
        raise ollama.urllib.error.HTTPError(request.full_url, 404, "nf", {}, None)

    with pytest.raises(ollama.OllamaError, match="HTTP 404"):
        ollama.delete_model("x", opener=not_found)
    with pytest.raises(ollama.OllamaError, match="No pude hablar"):
        ollama.delete_model("x", opener=lambda request, timeout: (_ for _ in ()).throw(ollama.urllib.error.URLError("down")))


def _configured(tmp_path: Path, monkeypatch, *, set_default: bool = True, extra: dict | None = None) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    if extra:
        (tmp_path / "opencode.json").write_text(json.dumps(extra))
    apply_change(tmp_path, _plan(tmp_path, set_default=set_default))


def test_removing_a_model_cleans_only_what_chxchx_wrote(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch)

    change = plan_remove_ollama_models(tmp_path, "qwen3-coder:30b")
    apply_change(tmp_path, change)

    assert json.loads((tmp_path / "opencode.json").read_text()) == {"$schema": opencode_local_config.SCHEMA_URL}
    assert not plan_remove_ollama_models(tmp_path, "qwen3-coder:30b").changed


def test_removal_keeps_user_models_provider_options_and_other_defaults(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch, set_default=False, extra={
        "model": "anthropic/claude-sonnet-5-5",
        "provider": {"ollama": {"options": {"baseURL": "http://gpu-box:11434/v1"}, "models": {"llama3": {"name": "L"}}}}})

    apply_change(tmp_path, plan_remove_ollama_models(tmp_path, "qwen3-coder:30b"))
    data = json.loads((tmp_path / "opencode.json").read_text())

    assert data["model"] == "anthropic/claude-sonnet-5-5" and set(data["provider"]["ollama"]["models"]) == {"llama3"}
    assert data["provider"]["ollama"]["options"]["baseURL"] == "http://gpu-box:11434/v1"


def test_removal_keeps_a_provider_the_user_customized(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch, set_default=False, extra={"provider": {"ollama": {"whitelist": ["a"]}}})

    apply_change(tmp_path, plan_remove_ollama_models(tmp_path, "qwen3-coder:30b"))

    provider = json.loads((tmp_path / "opencode.json").read_text())["provider"]["ollama"]
    assert provider["whitelist"] == ["a"] and provider["models"] == {}


def test_removal_without_a_config_changes_nothing(tmp_path: Path) -> None:
    change = plan_remove_ollama_models(tmp_path, "qwen3-coder:30b")
    assert not change.changed and not (tmp_path / "opencode.json").exists()


def _stub_remove(monkeypatch, models=(), running=True, installed=True):
    status = ollama.OllamaStatus(installed, running, "http://127.0.0.1:11434", "1",
                                 tuple(ollama.InstalledModel(name, 1) for name in models))
    monkeypatch.setattr(local_models_remove, "ollama_status", lambda: status)
    deleted: list[str] = []
    monkeypatch.setattr(local_models_remove, "delete_model", deleted.append)
    return deleted


def test_remove_deletes_variants_before_the_base_and_updates_the_config(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch)
    deleted = _stub_remove(monkeypatch, ("qwen3-coder:30b", "qwen3-coder:30b-ctx32k", "qwen3-coder:30b-ctx16k", "otro:1b"))
    events: list[dict] = []

    result = remove_local_model(tmp_path, "qwen3-coder:30b", events.append, confirmed=True)

    assert deleted == ["qwen3-coder:30b-ctx32k", "qwen3-coder:30b-ctx16k", "qwen3-coder:30b"]
    assert result.removed == tuple(deleted) and result.config_changed and result.backup_path is not None
    assert "model" not in json.loads((tmp_path / "opencode.json").read_text())
    assert [e["step"] for e in events if e["type"] == "step"] == ["delete", "delete", "delete", "config"]


def test_remove_dry_run_and_confirmation_never_touch_anything(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch)
    before = (tmp_path / "opencode.json").read_text()
    deleted = _stub_remove(monkeypatch, ("qwen3-coder:30b",), running=False)
    events: list[dict] = []

    remove_local_model(tmp_path, "qwen3-coder:30b", events.append, dry_run=True)

    assert deleted == [] and (tmp_path / "opencode.json").read_text() == before
    assert any(e["type"] == "preview" for e in events) and any("DRY RUN" in e.get("message", "") for e in events)
    _stub_remove(monkeypatch, ("qwen3-coder:30b",))
    with pytest.raises(LocalModelSetupError, match="--yes"):
        remove_local_model(tmp_path, "qwen3-coder:30b", lambda event: None)


def test_remove_explains_why_it_cannot_continue(tmp_path: Path, monkeypatch) -> None:
    _stub_remove(monkeypatch, installed=False)
    with pytest.raises(LocalModelSetupError, match="no está instalado"):
        remove_local_model(tmp_path, "qwen3-coder:30b", lambda event: None, confirmed=True)
    _stub_remove(monkeypatch, running=False)
    with pytest.raises(LocalModelSetupError, match="no responde"):
        remove_local_model(tmp_path, "qwen3-coder:30b", lambda event: None, confirmed=True)
    _stub_remove(monkeypatch)
    with pytest.raises(LocalModelSetupError, match="ni configurado"):
        remove_local_model(tmp_path, "qwen3-coder:30b", lambda event: None, confirmed=True)
    with pytest.raises(LocalModelSetupError, match="catálogo"):
        remove_local_model(tmp_path, "desconocido", lambda event: None, confirmed=True)


def test_a_failed_delete_stops_before_touching_the_config(tmp_path: Path, monkeypatch) -> None:
    _configured(tmp_path, monkeypatch)
    before = (tmp_path / "opencode.json").read_text()
    _stub_remove(monkeypatch, ("qwen3-coder:30b",))
    monkeypatch.setattr(local_models_remove, "delete_model",
                        lambda name: (_ for _ in ()).throw(ollama.OllamaError("HTTP 500")))

    with pytest.raises(LocalModelSetupError, match="HTTP 500"):
        remove_local_model(tmp_path, "qwen3-coder:30b", lambda event: None, confirmed=True)

    assert (tmp_path / "opencode.json").read_text() == before
