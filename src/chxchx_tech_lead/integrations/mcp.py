from __future__ import annotations

import json
import re
from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.project_config import memory_project_name
from chxchx_tech_lead.core.runner import executable, run


def server_commands(info: ProjectInfo) -> dict[str, list[str]]:
    memory_command = "basic-memory" if executable("basic-memory") else ("bm" if executable("bm") else "basic-memory")
    return {
        # Fijar el proyecto evita que Claude y Codex caigan en el proyecto
        # `main` y garantiza que compartan la misma memoria local.
        "basic-memory": [memory_command, "mcp", "--project", memory_project_name(info)],
        "serena": ["serena", "start-mcp-server", "--context", "ide-assistant", "--project-from-cwd"],
    }


def _contains_server(output: str, name: str) -> bool:
    return re.search(rf"(?i)(?<![\w-]){re.escape(name)}(?![\w-])", output) is not None


def _list_servers(client: str, allow_failure: bool = False):
    result = run([client, "mcp", "list"], dry_run=False)
    if result.returncode != 0 and not allow_failure:
        detail = result.stderr or result.stdout or "sin detalles"
        raise RuntimeError(f"No se pudo consultar MCP en '{client}': {detail}")
    return result


def _client_command(client: str, name: str, command: list[str]) -> list[str]:
    if client == "claude":
        return ["claude", "mcp", "add", name, "--", *command]
    if client == "codex":
        return ["codex", "mcp", "add", name, "--", *command]
    if client == "opencode":
        return ["opencode", "mcp", "add", name, "--", *command]
    raise ValueError(client)


def _remove_command(client: str, name: str) -> list[str]:
    return [client, "mcp", "remove", name]


def integrate(info: ProjectInfo, client: str, dry_run: bool = False, refresh: bool = False):
    if client not in {"claude", "codex", "opencode"}:
        raise ValueError(f"Cliente no soportado: {client}")
    if executable(client) is None:
        raise RuntimeError(f"No se encontró '{client}' en PATH")
    if executable("basic-memory") is None and executable("bm") is None:
        raise RuntimeError("Basic Memory no está instalado. Ejecuta `chxchx-tech install`.")
    if executable("serena") is None:
        raise RuntimeError("Serena no está instalado. Ejecuta `chxchx-tech install`.")

    listed = _list_servers(client, allow_failure=dry_run)
    known = listed.stdout + "\n" + listed.stderr if listed.returncode == 0 else ""
    results = []
    added_names = []
    for name, command in server_commands(info).items():
        if _contains_server(known, name):
            if refresh:
                removed = run(_remove_command(client, name), dry_run=dry_run)
                results.append(removed)
                if removed.returncode != 0 and not dry_run:
                    detail = removed.stderr or removed.stdout or "sin detalles"
                    raise RuntimeError(f"No se pudo actualizar el servidor MCP '{name}': {detail}")
            else:
                continue
        result = run(_client_command(client, name, command), dry_run=dry_run)
        results.append(result)
        if result.returncode == 0 and not dry_run:
            added_names.append(name)

    if added_names:
        verified = _list_servers(client)
        missing = [name for name in added_names if not _contains_server(verified.stdout + "\n" + verified.stderr, name)]
        if missing:
            raise RuntimeError(
                f"El cliente '{client}' no confirmó los servidores: {', '.join(missing)}"
            )
    return results


def write_opencode_example(info: ProjectInfo, dry_run: bool = False) -> bool:
    target = info.root / ".ai" / "integrations" / "opencode-mcp.example.json"
    content = {
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            "servers": {
                name: {"type": "local", "command": command}
                for name, command in server_commands(info).items()
            }
        },
    }
    expected = json.dumps(content, indent=2, ensure_ascii=False) + "\n"
    if target.exists() and target.read_text(encoding="utf-8") == expected:
        return False
    if not dry_run:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(expected, encoding="utf-8")
    return True
