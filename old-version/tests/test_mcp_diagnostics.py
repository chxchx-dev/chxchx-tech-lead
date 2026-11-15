import json
from pathlib import Path
from unittest.mock import patch

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.mcp_diagnostics import diagnose_project_mcp


COMMANDS = {
    "basic-memory": ["basic-memory", "mcp", "--project", "demo-123abc"],
    "serena": ["serena", "start-mcp-server", "--context", "ide-assistant", "--project-from-cwd"],
}


def _toml_server(name: str, command: list[str]) -> str:
    args = ", ".join(json.dumps(arg) for arg in command[1:])
    return f'[mcp_servers.{name}]\ncommand = {json.dumps(command[0])}\nargs = [{args}]\n'


def _diagnose(info: ProjectInfo, claude_path: Path, codex_path: Path):
    with patch(
        "chxchx_tech_lead.integrations.mcp_diagnostics.server_commands",
        return_value=COMMANDS,
    ):
        return diagnose_project_mcp(
            info,
            claude_config_path=claude_path,
            codex_user_config_path=codex_path,
        )


def test_diagnostics_recognize_correct_project_scoped_configuration(tmp_path: Path):
    info = ProjectInfo(tmp_path / "project", "project")
    info.root.mkdir()
    claude_path = tmp_path / "claude.json"
    claude_path.write_text(
        json.dumps(
            {
                "projects": {
                    str(info.root.resolve()): {
                        "mcpServers": {
                            name: {"command": command[0], "args": command[1:]}
                            for name, command in COMMANDS.items()
                        }
                    }
                }
            }
        ),
        encoding="utf-8",
    )
    codex_path = tmp_path / "codex.toml"
    codex_project = info.root / ".codex" / "config.toml"
    codex_project.parent.mkdir()
    codex_project.write_text("\n".join(_toml_server(name, command) for name, command in COMMANDS.items()), encoding="utf-8")

    results = _diagnose(info, claude_path, codex_path)

    assert len(results) == 4
    assert all(item.status == "OK" for item in results)
    assert {item.scope for item in results} == {"local", "proyecto"}


def test_diagnostics_flag_global_only_configuration(tmp_path: Path):
    info = ProjectInfo(tmp_path / "project", "project")
    info.root.mkdir()
    claude_path = tmp_path / "claude.json"
    claude_path.write_text(
        json.dumps(
            {
                "mcpServers": {
                    name: {"command": command[0], "args": command[1:]}
                    for name, command in COMMANDS.items()
                }
            }
        ),
        encoding="utf-8",
    )
    codex_path = tmp_path / "codex.toml"
    codex_path.write_text("\n".join(_toml_server(name, command) for name, command in COMMANDS.items()), encoding="utf-8")

    results = _diagnose(info, claude_path, codex_path)

    assert len(results) == 4
    assert all(item.status == "AVISO" for item in results)
    assert all(item.scope == "usuario/global" for item in results)


def test_project_configuration_takes_precedence_over_global_entries(tmp_path: Path):
    info = ProjectInfo(tmp_path / "project", "project")
    info.root.mkdir()
    claude_path = tmp_path / "claude.json"
    claude_path.write_text(
        json.dumps(
            {
                "mcpServers": {
                    name: {"command": "wrong-command", "args": []}
                    for name in COMMANDS
                },
                "projects": {
                    str(info.root.resolve()): {
                        "mcpServers": {
                            name: {"command": command[0], "args": command[1:]}
                            for name, command in COMMANDS.items()
                        }
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    codex_path = tmp_path / "codex.toml"
    codex_path.write_text(
        "\n".join(
            _toml_server(name, ["wrong-command"])
            for name in COMMANDS
        ),
        encoding="utf-8",
    )
    codex_project = info.root / ".codex" / "config.toml"
    codex_project.parent.mkdir()
    codex_project.write_text(
        "\n".join(_toml_server(name, command) for name, command in COMMANDS.items()),
        encoding="utf-8",
    )

    results = _diagnose(info, claude_path, codex_path)

    assert all(item.status == "OK" for item in results)
    assert {item.scope for item in results if item.client == "Claude Code"} == {"local"}
    assert {item.scope for item in results if item.client == "Codex"} == {"proyecto"}


def test_wrong_project_memory_is_reported_without_echoing_environment_values(tmp_path: Path):
    info = ProjectInfo(tmp_path / "project", "project")
    info.root.mkdir()
    (info.root / ".codex").mkdir()
    (info.root / ".codex" / "config.toml").write_text(
        '[mcp_servers.basic-memory]\n'
        'command = "basic-memory"\n'
        'args = ["mcp", "--project", "another-project"]\n'
        'env = { PRIVATE_TOKEN = "never-print-this" }\n',
        encoding="utf-8",
    )
    claude_path = tmp_path / "claude.json"
    codex_path = tmp_path / "codex-user.toml"

    results = _diagnose(info, claude_path, codex_path)
    codex_memory = next(item for item in results if item.client == "Codex" and item.server == "basic-memory")

    assert codex_memory.status == "ERROR"
    assert codex_memory.scope == "proyecto"
    assert "never-print-this" not in codex_memory.detail


def test_diagnostics_report_missing_configuration_and_malformed_files(tmp_path: Path):
    info = ProjectInfo(tmp_path / "project", "project")
    info.root.mkdir()
    claude_path = tmp_path / "claude.json"
    claude_path.write_text("{broken", encoding="utf-8")
    codex_path = tmp_path / "codex.toml"

    results = _diagnose(info, claude_path, codex_path)

    assert all(item.status == "ERROR" for item in results if item.client == "Claude Code")
    assert all(item.status == "FALTA" for item in results if item.client == "Codex")
