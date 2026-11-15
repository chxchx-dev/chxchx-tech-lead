from __future__ import annotations

import json
import os
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from chxchx_tech_lead.core.models import ProjectInfo

from .mcp import server_commands


@dataclass(frozen=True, slots=True)
class McpDiagnostic:
    client: str
    server: str
    status: str
    scope: str
    detail: str


def diagnose_project_mcp(
    info: ProjectInfo,
    *,
    claude_config_path: Path | None = None,
    codex_user_config_path: Path | None = None,
) -> list[McpDiagnostic]:
    """Check project MCP command/scope statically; never launches agents or servers."""
    commands = server_commands(info)
    claude_config_path = claude_config_path or _claude_config_path()
    codex_user_config_path = codex_user_config_path or _codex_user_config_path()

    claude_user = _read_json(claude_config_path)
    claude_project = _mapping_at(
        _mapping_at(claude_user.value, "projects").get(str(info.root.resolve())),
        "mcpServers",
    )
    claude_global = _mapping_at(claude_user.value, "mcpServers")
    claude_shared = _read_json(info.root / ".mcp.json")
    claude_team = _mapping_at(claude_shared.value, "mcpServers")

    codex_project_path = info.root / ".codex" / "config.toml"
    codex_project = _read_toml(codex_project_path)
    codex_user = _read_toml(codex_user_config_path)
    codex_project_servers = _mapping_at(codex_project.value, "mcp_servers")
    codex_user_servers = _mapping_at(codex_user.value, "mcp_servers")

    diagnostics: list[McpDiagnostic] = []
    for name, command in commands.items():
        diagnostics.append(
            _diagnose_entry(
                "Claude Code",
                name,
                command,
                (
                    ("local", claude_project.get(name), claude_user.error),
                    ("proyecto compartido", claude_team.get(name), claude_shared.error),
                    ("usuario/global", claude_global.get(name), claude_user.error),
                ),
            )
        )
        diagnostics.append(
            _diagnose_entry(
                "Codex",
                name,
                command,
                (
                    ("proyecto", codex_project_servers.get(name), codex_project.error),
                    ("usuario/global", codex_user_servers.get(name), codex_user.error),
                ),
            )
        )
    return diagnostics


@dataclass(frozen=True, slots=True)
class _ConfigRead:
    value: dict[str, Any]
    error: str | None = None


def _read_json(path: Path) -> _ConfigRead:
    try:
        raw = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    except (OSError, UnicodeError, json.JSONDecodeError):
        return _ConfigRead({}, "no se pudo leer o parsear")
    if not isinstance(raw, dict):
        return _ConfigRead({}, "raíz inválida")
    return _ConfigRead(raw)


def _read_toml(path: Path) -> _ConfigRead:
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    except (OSError, UnicodeError, tomllib.TOMLDecodeError):
        return _ConfigRead({}, "no se pudo leer o parsear")
    return _ConfigRead(raw)


def _mapping_at(value: Any, key: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        return {}
    result = value.get(key, {})
    return result if isinstance(result, dict) else {}


def _diagnose_entry(
    client: str,
    name: str,
    expected: list[str],
    scopes: tuple[tuple[str, Any, str | None], ...],
) -> McpDiagnostic:
    for scope, entry, _error in scopes:
        if entry is None:
            continue
        if not _entry_matches(entry, expected):
            return McpDiagnostic(
                client,
                name,
                "ERROR",
                scope,
                "configuración distinta a la esperada para este proyecto",
            )
        if scope == "usuario/global":
            return McpDiagnostic(
                client,
                name,
                "AVISO",
                scope,
                "solo está configurado globalmente; puede compartirse con otros proyectos",
            )
        if scope == "proyecto compartido":
            return McpDiagnostic(
                client,
                name,
                "OK",
                scope,
                "configuración válida; puede requerir aprobación del cliente",
            )
        return McpDiagnostic(client, name, "OK", scope, "comando y argumentos corresponden al proyecto")

    errors = [error for _scope, _entry, error in scopes if error]
    if errors:
        return McpDiagnostic(client, name, "ERROR", "configuración", errors[0])
    target = "claude" if client == "Claude Code" else "codex"
    return McpDiagnostic(
        client,
        name,
        "FALTA",
        "—",
        f"ejecuta `chxchx-tech integrate --client {target}`",
    )


def _entry_matches(entry: Any, expected: list[str]) -> bool:
    if not isinstance(entry, dict):
        return False
    command = entry.get("command")
    args = entry.get("args", [])
    return command == expected[0] and args == expected[1:] and isinstance(args, list)


def _claude_config_path() -> Path:
    config_dir = Path(os.getenv("CLAUDE_CONFIG_DIR", str(Path.home() / ".claude"))).expanduser()
    if config_dir.name == ".claude":
        return config_dir.parent / ".claude.json"
    return config_dir / ".claude.json"


def _codex_user_config_path() -> Path:
    config_dir = Path(os.getenv("CODEX_HOME", str(Path.home() / ".codex"))).expanduser()
    return config_dir / "config.toml"
