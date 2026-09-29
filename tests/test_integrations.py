from pathlib import Path
from unittest.mock import patch

import pytest

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.basic_memory import ensure_project
from chxchx_tech_lead.integrations.installers import install_tool
from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.integrations.mcp import integrate, write_opencode_example


def test_opencode_example_is_idempotent(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")

    assert write_opencode_example(info) is True
    assert write_opencode_example(info) is False


def test_memory_mcp_command_is_pinned_to_project(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"):
        result = write_opencode_example(info)

    assert result is True
    content = (tmp_path / ".ai" / "integrations" / "opencode-mcp.example.json").read_text(encoding="utf-8")
    assert '"--project"' in content
    assert "project-" in content

def test_install_dry_run_does_not_require_uv():
    with patch("chxchx_tech_lead.integrations.installers.executable", return_value=None):
        result = install_tool("basic-memory", dry_run=True)

    assert result.returncode == 0
    assert result.stdout == "DRY RUN"


def test_doctor_does_not_probe_docker():
    from chxchx_tech_lead.integrations.tools import TOOLS

    assert all(command != "docker" for _name, command in TOOLS)


def test_mcp_integration_skips_existing_servers(tmp_path: Path):
    calls = []

    def fake_run(command, dry_run=False, cwd=None):
        calls.append(command)
        return CommandResult(command, 0, "basic-memory\nserena", "")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chxchx_tech_lead.integrations.mcp.run", side_effect=fake_run
    ):
        results = integrate(ProjectInfo(tmp_path, "project"), "claude")

    assert results == []
    assert calls == [["claude", "mcp", "list"]]


def test_mcp_integration_verifies_new_servers(tmp_path: Path):
    calls = []
    list_count = 0

    def fake_run(command, dry_run=False, cwd=None):
        nonlocal list_count
        calls.append(command)
        if command == ["claude", "mcp", "list"]:
            list_count += 1
            output = "" if list_count == 1 else "basic-memory\nserena"
            return CommandResult(command, 0, output, "")
        return CommandResult(command, 0, "added", "")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chxchx_tech_lead.integrations.mcp.run", side_effect=fake_run
    ):
        results = integrate(ProjectInfo(tmp_path, "project"), "claude")

    assert len(results) == 2
    assert calls[0] == ["claude", "mcp", "list"]
    assert calls[-1] == ["claude", "mcp", "list"]
    additions = [call for call in calls if call[0:3] == ["claude", "mcp", "add"]]
    assert len(additions) == 2
    assert all(call[3:5] == ["--scope", "local"] for call in additions)


def test_mcp_integration_reports_list_failure(tmp_path: Path):
    def fake_run(command, dry_run=False, cwd=None):
        return CommandResult(command, 1, "", "client unavailable")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chxchx_tech_lead.integrations.mcp.run", side_effect=fake_run
    ), pytest.raises(RuntimeError, match="No se pudo consultar MCP"):
        integrate(ProjectInfo(tmp_path, "project"), "claude")


def test_codex_mcp_integration_is_project_scoped_and_preserves_other_config(tmp_path: Path):
    project_config = tmp_path / ".codex" / "config.toml"
    project_config.parent.mkdir()
    project_config.write_text('model = "keep-this"\n\n[mcp_servers.other]\ncommand = "other"\nargs = []\n', encoding="utf-8")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"):
        results = integrate(ProjectInfo(tmp_path, "project"), "codex")

    content = project_config.read_text(encoding="utf-8")
    assert len(results) == 2
    assert 'model = "keep-this"' in content
    assert "[mcp_servers.other]" in content
    assert '[mcp_servers.basic-memory]' in content
    assert "project-" in content
    assert "[mcp_servers.serena]" in content


def test_codex_mcp_integration_requires_refresh_for_conflicting_local_server(tmp_path: Path):
    project_config = tmp_path / ".codex" / "config.toml"
    project_config.parent.mkdir()
    project_config.write_text('[mcp_servers.basic-memory]\ncommand = "old"\nargs = []\n', encoding="utf-8")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"), pytest.raises(
        RuntimeError, match="--refresh"
    ):
        integrate(ProjectInfo(tmp_path, "project"), "codex")

    with patch("chxchx_tech_lead.integrations.mcp.executable", return_value="/fake"):
        integrate(ProjectInfo(tmp_path, "project"), "codex", refresh=True)

    assert 'command = "basic-memory"' in project_config.read_text(encoding="utf-8")


def test_existing_basic_memory_project_is_reported_as_skipped(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")

    with patch("chxchx_tech_lead.integrations.basic_memory.executable", return_value="/fake"), patch(
        "chxchx_tech_lead.integrations.basic_memory.run",
        return_value=CommandResult(["basic-memory", "project", "info"], 0, "{}", ""),
    ):
        result = ensure_project(info)

    assert result is not None
    assert result.skipped is True


def test_basic_memory_duplicate_add_is_confirmed_with_info(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")
    responses = iter(
        [
            CommandResult(["basic-memory", "project", "info"], 1, "", "not found"),
            CommandResult(["basic-memory", "project", "add"], 1, "", "already exists"),
            CommandResult(["basic-memory", "project", "info"], 0, "{}", ""),
        ]
    )

    with patch("chxchx_tech_lead.integrations.basic_memory.executable", return_value="/fake"), patch(
        "chxchx_tech_lead.integrations.basic_memory.run", side_effect=lambda *args, **kwargs: next(responses)
    ):
        result = ensure_project(info)

    assert result is not None
    assert result.skipped is True
