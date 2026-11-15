"""Read-only options of an agent for the chat panel (models, efforts, availability)."""

from __future__ import annotations

from typing import Any

from ..integrations.agent_catalog import agent_options
from ..integrations.agent_stream import provider_for
from .bridge_contracts import AGENT_OPTIONS_SCHEMA, SCHEMA_VERSION as BRIDGE_SCHEMA_VERSION
from .service import WorkspaceOperationError


def agent_options_payload(service: Any, agent_id: str) -> dict[str, Any]:
    """Read-only options for the chat panel; works before trust is granted."""
    inspection = service.inspect()
    agents = inspection.config.agents if inspection.config is not None else []
    config = next((item for item in agents if item.id == agent_id), None)
    if config is None or not isinstance(config.command, list) or not config.command:
        raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
    provider = provider_for(config.command)
    if provider is None:
        raise WorkspaceOperationError("El chat visual solo está disponible para agentes Codex, Claude Code y OpenCode")
    return {
        "schema": AGENT_OPTIONS_SCHEMA,
        "schema_version": BRIDGE_SCHEMA_VERSION,
        "agent": agent_id,
        **agent_options(provider, config.command[0]),
    }
