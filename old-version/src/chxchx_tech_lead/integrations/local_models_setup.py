"""Use case: make a local Ollama model usable from OpenCode in one project.

Steps, each reported as an event so a CLI or Studio can show progress:
download the model if missing, create the larger-context variant, and merge
it into the project's `opencode.json`. Downloading needs `confirmed=True`
because it spends disk and bandwidth; a dry run changes nothing.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .local_hardware import HardwareProfile
from .local_model_catalog import find_model
from .local_model_fit import DEFAULT_CONTEXT, assess
from .ollama import OllamaError, base_url, create_context_variant, ollama_status, pull_model
from .opencode_local_config import ConfigChange, OpenCodeConfigError, apply_change, plan_ollama_model

Emit = Callable[[dict[str, Any]], None]


class LocalModelSetupError(RuntimeError):
    """Raised when the setup cannot continue; the message says what to do."""


@dataclass(frozen=True, slots=True)
class SetupResult:
    model_ref: str
    context_model: str
    config_path: Path
    backup_path: Path | None
    changed: bool


def setup_local_model(
    root: Path,
    tag: str,
    hardware: HardwareProfile,
    emit: Emit,
    *,
    context: int = DEFAULT_CONTEXT,
    set_default: bool = False,
    confirmed: bool = False,
    dry_run: bool = False,
    force: bool = False,
) -> SetupResult:
    model = find_model(tag)
    if model is None:
        raise LocalModelSetupError(f"`{tag}` no está en el catálogo local; elige uno de `chxchx-tech local-models status`.")
    verdict = assess(model, hardware, context)
    if verdict.fit == "none" and not force:
        raise LocalModelSetupError(f"{model.label} no cabe en este equipo: {verdict.summary} Usa --force para intentarlo igual.")
    status = ollama_status()
    if not status.installed:
        raise LocalModelSetupError("Ollama no está instalado. Instálalo con el método oficial y vuelve a intentarlo.")
    if not status.running and not dry_run:
        raise LocalModelSetupError(f"Ollama no responde en {status.base_url}. Inicia la aplicación o `ollama serve`.")

    variant = f"{tag}-ctx{context // 1024}k"
    installed = {item.name for item in status.models}
    if tag not in installed:
        emit({"type": "step", "step": "download", "model": tag, "size_bytes": model.size_bytes})
        if dry_run:
            emit({"type": "notice", "message": f"DRY RUN: se descargarían ~{model.size_gb:.1f} GB de {tag}."})
        elif not confirmed:
            raise LocalModelSetupError(f"Descargar {tag} ocupa ~{model.size_gb:.1f} GB. Confirma la descarga (--yes) para continuar.")
        else:
            _pull(tag, emit)
    if variant not in installed:
        emit({"type": "step", "step": "context", "model": variant, "context": context})
        if dry_run:
            emit({"type": "notice", "message": f"DRY RUN: se crearía {variant} con contexto de {context} tokens."})
        else:
            try:
                create_context_variant(tag, context)
            except OllamaError as exc:
                raise LocalModelSetupError(str(exc)) from exc

    emit({"type": "step", "step": "config", "model": variant})
    try:
        change = plan_ollama_model(root, variant, label=f"{model.label} ({context // 1024}K)", context=context,
                                   endpoint=base_url(), set_default=set_default)
        backup = None if dry_run else apply_change(root, change)
    except (OpenCodeConfigError, OSError) as exc:
        raise LocalModelSetupError(str(exc)) from exc
    if dry_run and change.changed:
        emit({"type": "preview", "path": str(change.path), "content": change.after})
    return SetupResult(change.model_ref, variant, change.path, backup, change.changed)


def _pull(tag: str, emit: Emit) -> None:
    def report(status: str, completed: int, total: int) -> None:
        emit({"type": "progress", "model": tag, "status": status, "completed": completed, "total": total})

    try:
        pull_model(tag, report)
    except OllamaError as exc:
        raise LocalModelSetupError(f"La descarga de {tag} falló: {exc}") from exc
