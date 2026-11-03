"""Use case: remove a local model from Ollama and from the project's OpenCode config.

Removal covers the model and its `-ctx<N>k` variants. It is destructive for
disk (the user must download again), so it needs `confirmed=True`; a dry run
reports what would go and changes nothing. Config is edited like in setup:
additive-safe, backed up and atomic.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .local_model_catalog import find_model
from .local_models_setup import Emit, LocalModelSetupError
from .ollama import OllamaError, delete_model, ollama_status
from .opencode_local_config import OpenCodeConfigError, apply_change, plan_remove_ollama_models


@dataclass(frozen=True, slots=True)
class RemoveResult:
    removed: tuple[str, ...]
    config_path: Path
    config_changed: bool
    backup_path: Path | None


def remove_local_model(root: Path, tag: str, emit: Emit, *, confirmed: bool = False, dry_run: bool = False) -> RemoveResult:
    if find_model(tag) is None:
        raise LocalModelSetupError(f"`{tag}` no está en el catálogo local; solo quito los modelos que ChxChx instala.")
    status = ollama_status()
    if not status.installed:
        raise LocalModelSetupError("Ollama no está instalado.")
    if not status.running and not dry_run:
        raise LocalModelSetupError(f"Ollama no responde en {status.base_url}. Inicia la aplicación o `ollama serve`.")
    # Variants first: they point at the base weights, which go last.
    present = sorted((item.name for item in status.models if item.name.startswith(f"{tag}-ctx")), reverse=True)
    present += [item.name for item in status.models if item.name == tag]
    try:
        change = plan_remove_ollama_models(root, tag)
    except OpenCodeConfigError as exc:
        raise LocalModelSetupError(str(exc)) from exc
    if not present and not change.changed:
        raise LocalModelSetupError(f"{tag} no está descargado ni configurado en este proyecto.")
    if not dry_run and not confirmed:
        raise LocalModelSetupError(f"Quitar {tag} borra el modelo de Ollama y de opencode.json. Confirma con --yes para continuar.")

    for name in present:
        emit({"type": "step", "step": "delete", "model": name})
        if dry_run:
            emit({"type": "notice", "message": f"DRY RUN: se borraría {name} de Ollama."})
            continue
        try:
            delete_model(name)
        except OllamaError as exc:
            raise LocalModelSetupError(str(exc)) from exc
    emit({"type": "step", "step": "config", "model": tag})
    try:
        backup = None if dry_run else apply_change(root, change)
    except OSError as exc:
        raise LocalModelSetupError(str(exc)) from exc
    if dry_run and change.changed:
        emit({"type": "preview", "path": str(change.path), "content": change.after})
    return RemoveResult(tuple(present), change.path, change.changed, backup)
