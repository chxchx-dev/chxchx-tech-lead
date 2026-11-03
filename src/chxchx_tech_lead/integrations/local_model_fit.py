"""Estimate whether a local model fits a machine and how fast it will run.

The figures are estimates: weights plus a runtime margin plus a key/value
cache that grows with the context window. They are meant to avoid obviously
bad downloads, not to promise speed.
"""

from __future__ import annotations

from dataclasses import dataclass

from .local_hardware import HardwareProfile
from .local_model_catalog import CATALOG, GIB, CatalogModel

DEFAULT_CONTEXT = 32768
_WEIGHT_MARGIN = 1.05  # the download is already quantized: memory use is close to it
_KV_MIB_PER_1K_CONTEXT = 64  # ~2 GB at 32K; a 30B grouped-query model uses ~3 GB
_GPU_BUDGET = 0.95
_RAM_BUDGET = 0.7
_PARTIAL_RAM_BUDGET = 0.6
_CPU_MAX_GB = 10.0  # larger models on CPU are too slow to be a useful agent
_FIT_ORDER = ("gpu", "partial", "cpu")


@dataclass(frozen=True, slots=True)
class Assessment:
    model: CatalogModel
    fit: str  # "gpu" | "partial" | "cpu" | "none"
    required_bytes: int
    disk_ok: bool
    summary: str


def required_bytes(model: CatalogModel, context: int = DEFAULT_CONTEXT) -> int:
    return int(model.size_bytes * _WEIGHT_MARGIN + context / 1024 * _KV_MIB_PER_1K_CONTEXT * 1024**2)


def assess(model: CatalogModel, hardware: HardwareProfile, context: int = DEFAULT_CONTEXT) -> Assessment:
    need = required_bytes(model, context)
    accelerated = hardware.accelerated_bytes
    available = hardware.available_ram_bytes
    if accelerated and need <= accelerated * _GPU_BUDGET and (not hardware.unified_memory or need <= available):
        fit, summary = "gpu", "Cabe en la GPU: velocidad completa."
    elif accelerated and not hardware.unified_memory and need <= accelerated + available * _PARTIAL_RAM_BUDGET:
        fit, summary = "partial", "Se reparte entre GPU y RAM: funciona, pero más lento."
    elif need <= available * _RAM_BUDGET and model.size_gb <= _CPU_MAX_GB:
        fit, summary = "cpu", "Solo CPU: lento, aceptable para tareas pequeñas."
    else:
        fit, summary = "none", f"Necesita ~{need / GIB:.0f} GB de memoria y no cabe en este equipo."
    disk_ok = hardware.disk_free_bytes >= model.size_bytes * _WEIGHT_MARGIN
    if fit != "none" and not disk_ok:
        summary += f" Falta espacio en disco (libres {hardware.disk_free_bytes / GIB:.0f} GB)."
    return Assessment(model, fit, need, disk_ok, summary)


def recommend(hardware: HardwareProfile, context: int = DEFAULT_CONTEXT) -> list[Assessment]:
    """All catalog models assessed, with the best usable one first."""
    assessed = [assess(model, hardware, context) for model in CATALOG]
    usable = [item for item in assessed if item.fit != "none" and item.disk_ok]
    best = next((item for fit in _FIT_ORDER for item in usable if item.fit == fit), None)
    if best is None:
        return assessed
    return [best, *(item for item in assessed if item is not best)]
