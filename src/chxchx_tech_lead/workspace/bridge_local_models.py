"""Read-only snapshot for Studio's local-models tab (`chxchx.local-models`)."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from ..integrations.agent_catalog import INSTALL_HINTS, executable_available
from ..integrations.local_hardware import HardwareProfile, detect_hardware
from ..integrations.local_model_fit import DEFAULT_CONTEXT, recommend
from ..integrations.ollama import install_instructions, ollama_status
from ..integrations.opencode_local_config import configured_ollama_models
from .bridge_contracts import LOCAL_MODELS_SCHEMA, SCHEMA_VERSION
from .resources import read_system_resources


def current_hardware() -> HardwareProfile:
    system = read_system_resources()
    return detect_hardware(system.total_bytes, system.available_bytes)


def local_models_payload(project_path: Path) -> dict[str, Any]:
    hardware = current_hardware()
    ollama = ollama_status()
    installed = {item.name for item in ollama.models}
    loaded = {item.name for item in ollama.loaded}
    configured = configured_ollama_models(project_path)
    return {
        "schema": LOCAL_MODELS_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "hardware": {
            "total_ram_bytes": hardware.total_ram_bytes,
            "available_ram_bytes": hardware.available_ram_bytes,
            "cpu_cores": hardware.cpu_cores,
            "disk_free_bytes": hardware.disk_free_bytes,
            "models_dir": str(hardware.models_dir),
            "accelerator": hardware.accelerator,
            "accelerated_bytes": hardware.accelerated_bytes,
        },
        "ollama": {
            "installed": ollama.installed,
            "running": ollama.running,
            "version": ollama.version,
            "base_url": ollama.base_url,
            "install": install_instructions(),
            "models": [{"name": item.name, "size_bytes": item.size_bytes} for item in ollama.models],
            "loaded": [{"name": item.name, "size_bytes": item.size_bytes, "vram_bytes": item.vram_bytes}
                       for item in ollama.loaded],
        },
        "opencode": {
            "installed": executable_available("opencode"),
            "install_hint": INSTALL_HINTS["OpenCode"],
            "configured_models": configured,
        },
        "context": DEFAULT_CONTEXT,
        "models": [
            {
                "tag": item.model.tag,
                "label": item.model.label,
                "size_bytes": item.model.size_bytes,
                "context_k": item.model.context_k,
                "note": item.model.note,
                "fit": item.fit,
                "required_bytes": item.required_bytes,
                "disk_ok": item.disk_ok,
                "summary": item.summary,
                "recommended": index == 0 and item.fit != "none" and item.disk_ok,
                "downloaded": item.model.tag in installed,
                "loaded": any(name == item.model.tag or name.startswith(f"{item.model.tag}-ctx") for name in loaded),
                "configured": any(name.startswith(f"{item.model.tag}-ctx") for name in configured),
            }
            for index, item in enumerate(recommend(hardware))
        ],
    }
