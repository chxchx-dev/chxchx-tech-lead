from pathlib import Path
from unittest.mock import patch

import pytest

from chichan_tech_lead.core.models import ProjectInfo
from chichan_tech_lead.integrations.basic_memory import ensure_project
from chichan_tech_lead.integrations.installers import install_tool
from chichan_tech_lead.core.runner import CommandResult
from chichan_tech_lead.integrations.mcp import integrate, write_opencode_example


def test_opencode_example_is_idempotent(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")

    assert write_opencode_example(info) is True
    assert write_opencode_example(info) is False

def test_install_dry_run_does_not_require_uv():
    with patch("chichan_tech_lead.integrations.installers.executable", return_value=None):
        result = install_tool("basic-memory", dry_run=True)

    assert result.returncode == 0
    assert result.stdout == "DRY RUN"


def test_mcp_integration_skips_existing_servers(tmp_path: Path):
    calls = []

    def fake_run(command, dry_run=False):
        calls.append(command)
        return CommandResult(command, 0, "basic-memory\nserena", "")

    with patch("chichan_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chichan_tech_lead.integrations.mcp.run", side_effect=fake_run
    ):
        results = integrate(ProjectInfo(tmp_path, "project"), "claude")

    assert results == []
    assert calls == [["claude", "mcp", "list"]]


def test_mcp_integration_verifies_new_servers(tmp_path: Path):
    calls = []
    list_count = 0

    def fake_run(command, dry_run=False):
        nonlocal list_count
        calls.append(command)
        if command == ["claude", "mcp", "list"]:
            list_count += 1
            output = "" if list_count == 1 else "basic-memory\nserena"
            return CommandResult(command, 0, output, "")
        return CommandResult(command, 0, "added", "")

    with patch("chichan_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chichan_tech_lead.integrations.mcp.run", side_effect=fake_run
    ):
        results = integrate(ProjectInfo(tmp_path, "project"), "claude")

    assert len(results) == 2
    assert calls[0] == ["claude", "mcp", "list"]
    assert calls[-1] == ["claude", "mcp", "list"]


def test_mcp_integration_reports_list_failure(tmp_path: Path):
    def fake_run(command, dry_run=False):
        return CommandResult(command, 1, "", "client unavailable")

    with patch("chichan_tech_lead.integrations.mcp.executable", return_value="/fake"), patch(
        "chichan_tech_lead.integrations.mcp.run", side_effect=fake_run
    ), pytest.raises(RuntimeError, match="No se pudo consultar MCP"):
        integrate(ProjectInfo(tmp_path, "project"), "claude")


def test_existing_basic_memory_project_is_reported_as_skipped(tmp_path: Path):
    info = ProjectInfo(tmp_path, "project")

    with patch("chichan_tech_lead.integrations.basic_memory.executable", return_value="/fake"), patch(
        "chichan_tech_lead.integrations.basic_memory.run",
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

    with patch("chichan_tech_lead.integrations.basic_memory.executable", return_value="/fake"), patch(
        "chichan_tech_lead.integrations.basic_memory.run", side_effect=lambda *args, **kwargs: next(responses)
    ):
        result = ensure_project(info)

    assert result is not None
    assert result.skipped is True
