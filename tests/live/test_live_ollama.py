"""Ollama assumptions against a real server. Downloads are opt-in.

`CHXCHX_LIVE_PULL=qwen2.5-coder:3b` (any catalog tag) enables the round trip that
downloads a model, creates the context variant, configures OpenCode and removes it.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

from chxchx_tech_lead.integrations import ollama
from chxchx_tech_lead.integrations.local_models_remove import remove_local_model
from chxchx_tech_lead.integrations.local_models_setup import setup_local_model
from chxchx_tech_lead.workspace.bridge_local_models import current_hardware


@pytest.fixture
def server(ollama_binary: str) -> ollama.OllamaStatus:
    status = ollama.ollama_status()
    if not status.running:
        pytest.skip(f"Ollama no responde en {status.base_url}; inicia la aplicación o `ollama serve`")
    return status


def _cli_models(binary: str) -> set[str]:
    out = subprocess.run([binary, "list"], capture_output=True, text=True, timeout=30, check=True).stdout
    return {line.split()[0] for line in out.splitlines()[1:] if line.strip()}


def test_status_agrees_with_the_cli_and_reports_a_version(server: ollama.OllamaStatus, ollama_binary: str) -> None:
    """Settles: /api/version and /api/tags have the shape the adapter parses."""
    assert server.version and {m.name for m in server.models} == _cli_models(ollama_binary)
    assert all(m.size_bytes > 0 for m in server.models)


def test_loaded_models_have_typed_fields(server: ollama.OllamaStatus) -> None:
    """Settles: /api/ps entries carry name, size and size_vram."""
    for model in server.loaded:
        assert model.name and model.size_bytes > 0 and model.vram_bytes >= 0


def test_deleting_an_unknown_model_is_a_clear_error(server: ollama.OllamaStatus) -> None:
    """Settles: DELETE /api/delete on a missing model answers an HTTP error, not success."""
    with pytest.raises(ollama.OllamaError, match="HTTP 4"):
        ollama.delete_model("chxchx-no-existe:0b")


@pytest.mark.skipif(not os.environ.get("CHXCHX_LIVE_PULL"), reason="define CHXCHX_LIVE_PULL=<etiqueta del catálogo> para descargar un modelo")
def test_setup_and_remove_round_trip(server: ollama.OllamaStatus, ollama_binary: str, tmp_path: Path) -> None:
    """Settles: pull progress, `ollama create` with a Modelfile, the opencode.json provider and removal."""
    tag = os.environ["CHXCHX_LIVE_PULL"]
    variant = f"{tag}-ctx32k"
    existed = tag in {m.name for m in server.models}
    events: list[dict] = []
    try:
        result = setup_local_model(tmp_path, tag, current_hardware(), events.append, confirmed=True, set_default=True, force=True)

        assert any(e["type"] == "progress" for e in events) or existed
        assert {tag, variant} <= _cli_models(ollama_binary)
        config = json.loads((tmp_path / "opencode.json").read_text(encoding="utf-8"))
        assert config["model"] == result.model_ref == f"ollama/{variant}"
        shown = subprocess.run([ollama_binary, "show", variant], capture_output=True, text=True, timeout=30, check=True).stdout
        assert "32768" in shown, "the context variant should carry num_ctx 32768"
        opencode = subprocess.run(["opencode", "models"], capture_output=True, text=True, timeout=60, check=False) if _has("opencode") else None
        if opencode is not None and opencode.returncode == 0:
            assert f"ollama/{variant}" in opencode.stdout, "opencode does not list the configured Ollama model"
    finally:
        if existed:  # never delete a model the user already had; only the variant we made
            ollama.delete_model(variant)
        else:
            remove_local_model(tmp_path, tag, lambda event: None, confirmed=True)

    assert variant not in _cli_models(ollama_binary)


def _has(name: str) -> bool:
    from shutil import which

    return which(name) is not None
