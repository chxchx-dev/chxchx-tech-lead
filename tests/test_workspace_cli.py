from pathlib import Path
import importlib.util

from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.workspace.error_cache import record_error


runner = CliRunner()


def test_workspace_status_is_read_only(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["workspace", "status", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "Trust: no" in result.stdout
    assert "Workspace: STOPPED" in result.stdout
    assert not (tmp_path / "global").exists()


def test_workspace_trust_dry_run_does_not_write_global_state(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    config = project / ".ai" / "chxchx-tech.toml"
    config.parent.mkdir()
    config.write_text("version = 2\n", encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["workspace", "trust", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "DRY RUN" in result.stdout
    assert not (tmp_path / "global").exists()


def test_workspace_trust_allows_uninitialized_project(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["workspace", "trust", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "proyecto confiable" in result.stdout
    assert (tmp_path / "global" / "trusted_projects.json").exists()


def test_workspace_errors_lists_only_the_selected_project(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    other = tmp_path / "other"
    project.mkdir()
    other.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    record_error(project="project", project_path=project, operation="agent add", message="Missing config")
    record_error(project="other", project_path=other, operation="build", message="Other error")

    result = runner.invoke(app, ["workspace", "errors", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "agent add: Missing config" in result.stdout
    assert "Other error" not in result.stdout
    assert "errors.jsonl" in result.stdout


def test_process_list_reports_empty_configuration(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["process", "list", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "No hay procesos configurados" in result.stdout
    assert not (tmp_path / "global").exists()


def test_resources_reports_system_without_starting_processes(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["resources", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "Resource Manager" in result.stdout
    assert "RAM" in result.stdout


def test_doctor_reports_uninitialized_workspace(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    result = runner.invoke(app, ["doctor", str(project)])

    assert result.exit_code == 0, result.stdout
    assert ".ai/chxchx-tech.toml no existe" in result.stdout


def test_tui_reports_missing_optional_textual_dependency(tmp_path: Path):
    if importlib.util.find_spec("textual") is not None:
        return

    result = runner.invoke(app, ["tui", str(tmp_path)])

    assert result.exit_code == 1
    assert "Textual no está instalado" in result.stdout


def test_workspace_memory_prune_dry_run_lists_noise(tmp_path: Path, monkeypatch):
    from chxchx_tech_lead.integrations.explicit_memory import ExplicitMemory, persist_explicit_memories

    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    persist_explicit_memories(
        project, [ExplicitMemory("Codex", "s1", "2026-10-01", "Revisa el uso de memoria RAM.")]
    )
    memory = project / ".ai" / "memory" / "PROJECT_MEMORY.md"
    before = memory.read_text(encoding="utf-8")

    result = runner.invoke(app, ["workspace", "memory-prune", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "se retirarían 1 entrada(s)" in result.stdout
    assert "memoria RAM" in result.stdout
    assert memory.read_text(encoding="utf-8") == before


def test_workspace_context_compact_dry_run_lists_sections(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    handoff = project / ".ai" / "HANDOFF.md"
    handoff.parent.mkdir(parents=True)
    handoff.write_text("# Handoff\n\n" + "".join(f"## Paso {i}\n\n- x\n\n" for i in range(8, 0, -1)), encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    before = handoff.read_text(encoding="utf-8")

    result = runner.invoke(app, ["workspace", "context-compact", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "2 sección(es) se archivarían" in result.stdout
    assert "Paso 1" in result.stdout
    assert handoff.read_text(encoding="utf-8") == before
