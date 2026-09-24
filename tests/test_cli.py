from pathlib import Path

from typer.testing import CliRunner

from chichan_tech_lead.cli import app
from chichan_tech_lead.core.registry import register_project
from chichan_tech_lead.core.models import ProjectInfo


runner = CliRunner()


def test_version_command():
    result = runner.invoke(app, ["version"])

    assert result.exit_code == 0
    assert "chichan-tech-lead" in result.stdout


def test_init_dry_run_does_not_write_project_or_global_state(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))

    result = runner.invoke(app, ["init", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "No se realizaron cambios" in result.stdout
    assert not (project / ".ai").exists()
    assert not (project / "AGENTS.md").exists()
    assert not global_home.exists()


def test_init_is_stable_on_second_run(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHICHAN_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chichan_tech_lead.cli.basic_memory_available", lambda: False)

    first = runner.invoke(app, ["init", str(project)])
    second = runner.invoke(app, ["init", str(project)])

    assert first.exit_code == 0, first.stdout
    assert second.exit_code == 0, second.stdout
    assert "Todo está actualizado" in second.stdout
    assert (project / ".ai" / "chichan.toml").exists()
    assert (project / ".ai" / "integrations" / "opencode-mcp.example.json").exists()


def test_install_dry_run_reports_commands_without_uv(monkeypatch):
    monkeypatch.setattr("chichan_tech_lead.integrations.installers.executable", lambda name: None)

    result = runner.invoke(app, ["install", "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert result.stdout.count("DRY RUN") == 2


def test_projects_list_and_sync(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHICHAN_HOME", str(tmp_path / "global"))
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
    monkeypatch.setenv("CHICHAN_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chichan_tech_lead.cli.basic_memory_available", lambda: True)
    monkeypatch.setattr(
        "chichan_tech_lead.cli.ensure_memory_project",
        lambda info, dry_run=False: __import__(
            "chichan_tech_lead.core.runner", fromlist=["CommandResult"]
        ).CommandResult(["basic-memory", "project", "info"], 0, "{}", "", skipped=True),
    )

    result = runner.invoke(app, ["init", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "Basic Memory project:" not in result.stdout
