"""Opt-in project configuration for supported coding-agent launchers."""

from __future__ import annotations

import json
import os
import stat
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

from ..core.backup import backup_project
from .models import WorkspaceConfig, WorkspaceConfigError


class AgentSetupError(ValueError):
    """Raised when an agent cannot be added safely to the project config."""


@dataclass(frozen=True, slots=True)
class AgentProvider:
    id: str
    label: str
    command: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class AgentSetupResult:
    path: Path
    provider: AgentProvider
    changed: bool
    dry_run: bool
    backup_path: Path | None = None
    preview: str = ""


AGENT_PROVIDERS = (
    AgentProvider("opencode", "OpenCode", ("opencode",)),
    AgentProvider("ollama-opencode", "OpenCode con Ollama local", ("ollama", "launch", "opencode")),
)


def supported_agent_providers() -> tuple[AgentProvider, ...]:
    return AGENT_PROVIDERS


def add_agent(root: Path, provider_id: str, *, dry_run: bool = False) -> AgentSetupResult:
    """Append a supported agent to an existing v2 config, with backup on write."""
    providers = {provider.id: provider for provider in AGENT_PROVIDERS}
    provider = providers.get(provider_id)
    if provider is None:
        supported = ", ".join(providers)
        raise AgentSetupError(f"Agente no compatible: {provider_id}. Opciones: {supported}")

    project_root = root.expanduser().resolve()
    config_path = project_root / ".ai" / "chxchx-tech.toml"
    try:
        original = config_path.read_bytes()
        config = tomllib.loads(original.decode("utf-8"))
    except FileNotFoundError as exc:
        raise AgentSetupError("No existe .ai/chxchx-tech.toml; ejecuta `chxchx-tech init` primero.") from exc
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        raise AgentSetupError(f"No pude leer la configuración del proyecto: {exc}") from exc

    version = config.get("version")
    if isinstance(version, bool) or not isinstance(version, int) or version != 2:
        raise AgentSetupError("`agent add` requiere una configuración v2; ejecuta `chxchx-tech init` primero.")
    workspace = config.get("workspace")
    try:
        valid_workspace = WorkspaceConfig.from_mapping(workspace, project_root=project_root)
    except WorkspaceConfigError as exc:
        raise AgentSetupError(f"La configuración del workspace no es válida: {exc}") from exc

    existing = next((item for item in valid_workspace.agents if item.id == provider.id), None)
    if existing is not None:
        if existing.command == list(provider.command):
            return AgentSetupResult(config_path, provider, False, dry_run, preview="Agente ya configurado.")
        raise AgentSetupError(
            f"Ya existe el ID `{provider.id}` con otro comando; no se sobrescribió la configuración."
        )

    block = _provider_block(provider)
    if dry_run:
        return AgentSetupResult(config_path, provider, True, True, preview=block)

    try:
        backup = backup_project(project_root)
    except OSError as exc:
        raise AgentSetupError(f"No pude crear el backup previo; no se realizaron cambios: {exc}") from exc
    if backup is None:
        raise AgentSetupError("No pude crear el backup previo de la configuración; no se realizaron cambios.")
    try:
        _atomic_append(config_path, original, block)
    except OSError as exc:
        raise AgentSetupError(f"La escritura falló; el backup previo está en {backup}: {exc}") from exc
    return AgentSetupResult(config_path, provider, True, False, backup_path=backup, preview=block)


def _provider_block(provider: AgentProvider) -> str:
    command = json.dumps(list(provider.command), ensure_ascii=False)
    return (
        f"[[workspace.agents]]\n"
        f"id = {json.dumps(provider.id, ensure_ascii=False)}\n"
        f"command = {command}\n"
        f'cwd = "."\n'
        f"auto_start = false\n"
    )


def _atomic_append(path: Path, original: bytes, block: str) -> None:
    separator = b"" if not original or original.endswith(b"\n") else b"\n"
    updated = original + separator + b"\n" + block.encode("utf-8")
    mode = stat.S_IMODE(path.stat().st_mode)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(updated)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
