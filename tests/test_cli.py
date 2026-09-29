from pathlib import Path

from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.core.registry import register_project
from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.mcp_diagnostics import McpDiagnostic


runner = CliRunner()


def test_version_command():
    result = runner.invoke(app, ["version"])

    assert result.exit_code == 0
    assert "chxchx-tech-lead" in result.stdout


def test_doctor_reports_project_scoped_mcp_diagnostics_without_mutating(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.check_tools", lambda: [])
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.setup.diagnose_project_mcp",
        lambda _info: [
            McpDiagnostic("Codex", "basic-memory", "AVISO", "usuario/global", "solo global")
        ],
    )

    result = runner.invoke(app, ["doctor", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "MCP POR PROYECTO" in result.stdout
    assert "basic-memory" in result.stdout
    assert "AVISO" in result.stdout
    assert "no inicia agentes/servidores" in result.stdout
    assert not (project / ".codex").exists()
    assert not (project / ".mcp.json").exists()


def test_init_dry_run_does_not_write_project_or_global_state(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))

    result = runner.invoke(app, ["init", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "No se realizaron cambios" in result.stdout
    assert not (project / ".ai").exists()
    assert not (project / "AGENTS.md").exists()
    assert not global_home.exists()


def test_init_is_stable_on_second_run(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.basic_memory_available", lambda: False)

    first = runner.invoke(app, ["init", str(project)])
    second = runner.invoke(app, ["init", str(project)])

    assert first.exit_code == 0, first.stdout
    assert second.exit_code == 0, second.stdout
    assert "Todo está actualizado" in second.stdout
    assert (project / ".ai" / "chxchx-tech.toml").exists()
    assert (project / ".ai" / "integrations" / "opencode-mcp.example.json").exists()


def test_install_dry_run_reports_commands_without_uv(monkeypatch):
    monkeypatch.setattr("chxchx_tech_lead.integrations.installers.executable", lambda name: None)

    result = runner.invoke(app, ["install", "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert result.stdout.count("DRY RUN") == 2


def test_projects_list_and_sync(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    register_project(ProjectInfo(project, "project"))

    listed = runner.invoke(app, ["projects", "list"])
    synced = runner.invoke(app, ["projects", "sync"])

    assert listed.exit_code == 0, listed.stdout
    assert "project" in listed.stdout
    assert synced.exit_code == 0, synced.stdout
    assert (project / "AGENTS.md").exists()
    assert (project / "CLAUDE.md").exists()


def test_init_does_not_warn_for_existing_basic_memory_project(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.basic_memory_available", lambda: True)
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.setup.ensure_memory_project",
        lambda info, dry_run=False: __import__(
            "chxchx_tech_lead.core.runner", fromlist=["CommandResult"]
        ).CommandResult(["basic-memory", "project", "info"], 0, "{}", "", skipped=True),
    )

    result = runner.invoke(app, ["init", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "Basic Memory project:" not in result.stdout


def test_init_minimal_only_creates_compact_project_context(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.basic_memory_available", lambda: False)

    result = runner.invoke(app, ["init", str(project), "--minimal"])

    assert result.exit_code == 0, result.stdout
    assert (project / ".ai" / "chxchx-tech.toml").exists()
    assert (project / ".ai" / "memory").is_dir()
    assert not (project / "AGENTS.md").exists()
    assert not (project / "CLAUDE.md").exists()
    assert not (project / "docs" / "adr").exists()
