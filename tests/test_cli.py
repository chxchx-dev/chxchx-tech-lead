from pathlib import Path

from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.core.backup import backup_project
from chxchx_tech_lead.core.registry import register_project
from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.mcp_diagnostics import McpDiagnostic


runner = CliRunner()


def test_version_command():
    result = runner.invoke(app, ["version"])

    assert result.exit_code == 0
    assert "chxchx-tech-lead" in result.stdout


def test_editor_setup_dry_run_and_generation_are_idempotent(tmp_path: Path):
    project = tmp_path / "project"
    project.mkdir()

    preview = runner.invoke(app, ["editor", "setup", str(project), "--dry-run", "--no-open"])
    assert preview.exit_code == 0, preview.stdout
    assert "DRY RUN" in preview.stdout
    assert not (project / ".ai").exists()

    generated = runner.invoke(app, ["editor", "setup", str(project), "--no-open"])
    unchanged = runner.invoke(app, ["editor", "setup", str(project), "--no-open"])
    assert generated.exit_code == 0, generated.stdout
    assert "generado" in generated.stdout
    assert unchanged.exit_code == 0, unchanged.stdout
    assert "sin cambios" in unchanged.stdout


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
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: False)

    first = runner.invoke(app, ["init", str(project)])
    second = runner.invoke(app, ["init", str(project)])

    assert first.exit_code == 0, first.stdout
    assert second.exit_code == 0, second.stdout
    assert "Todo está actualizado" in second.stdout
    assert (project / ".ai" / "chxchx-tech.toml").exists()
    assert (project / ".ai" / "integrations" / "opencode-mcp.example.json").exists()


def test_doctor_checks_config_after_init(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: False)
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.check_tools", lambda: [])
    monkeypatch.setattr("chxchx_tech_lead.commands.setup.diagnose_project_mcp", lambda _info: [])

    initialized = runner.invoke(app, ["init", str(project)])
    diagnosed = runner.invoke(app, ["doctor", str(project)])

    assert initialized.exit_code == 0, initialized.stdout
    assert diagnosed.exit_code == 0, diagnosed.stdout
    assert ".ai/chxchx-tech.toml válido" in diagnosed.stdout


def test_install_dry_run_reports_commands_without_uv(monkeypatch):
    monkeypatch.setattr("chxchx_tech_lead.integrations.installers.executable", lambda name: None)

    result = runner.invoke(app, ["install", "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert result.stdout.count("DRY RUN") == 2


def test_setup_dry_run_preserves_three_stage_flow_without_writes(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: False)
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.setup.install_tool",
        lambda tool, dry_run=False: CommandResult(
            ["uv", "tool", "install", tool], 0, "DRY RUN", ""
        ),
    )
    integrations = []
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.integrations.integrate_mcp",
        lambda info, client, dry_run=False, refresh=False: integrations.append(
            (client, dry_run, refresh)
        ) or [],
    )

    result = runner.invoke(app, ["setup", str(project), "--dry-run", "--minimal"])

    assert result.exit_code == 0, result.stdout
    assert "1/3 Herramientas base" in result.stdout
    assert "2/3 Inicialización del proyecto" in result.stdout
    assert "3/3 Integraciones MCP" in result.stdout
    assert "No se realizaron cambios" in result.stdout
    assert integrations == [("claude", True, False), ("codex", True, False), ("opencode", True, False)]
    assert not (project / ".ai").exists()
    assert not global_home.exists()


def test_sync_dry_run_does_not_write_project_or_global_state(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))

    result = runner.invoke(app, ["sync", str(project), "--dry-run"])

    assert result.exit_code == 0, result.stdout
    assert "No se realizaron cambios" in result.stdout
    assert not (project / "AGENTS.md").exists()
    assert not (project / "CLAUDE.md").exists()
    assert not global_home.exists()


def test_rollback_restores_latest_backup_through_cli(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))
    (project / "AGENTS.md").write_text("versión guardada", encoding="utf-8")
    backup_project(project)
    (project / "AGENTS.md").write_text("versión nueva", encoding="utf-8")

    result = runner.invoke(app, ["rollback", str(project)])

    assert result.exit_code == 0, result.stdout
    assert "Restaurado desde" in result.stdout
    assert (project / "AGENTS.md").read_text(encoding="utf-8") == "versión guardada"


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
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: True)
    monkeypatch.setattr(
        "chxchx_tech_lead.core.bootstrap.ensure_memory_project",
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
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: False)

    result = runner.invoke(app, ["init", str(project), "--minimal"])

    assert result.exit_code == 0, result.stdout
    assert (project / ".ai" / "chxchx-tech.toml").exists()
    assert (project / ".ai" / "memory").is_dir()
    assert not (project / "AGENTS.md").exists()
    assert not (project / "CLAUDE.md").exists()
    assert not (project / "docs" / "adr").exists()

def test_skill_cli_enable_and_sync_are_explicit_and_previewable(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    (project / "package.json").write_text(
        '{"dependencies":{"next":"^15","react":"^19"}}',
        encoding="utf-8",
    )
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))

    recommended = runner.invoke(app, ["skill", "recommend", str(project)])
    preview = runner.invoke(app, ["skill", "enable", "nextjs", str(project), "--dry-run"])

    assert recommended.exit_code == 0, recommended.stdout
    assert "nextjs" in recommended.stdout
    assert "stack detectado: nextjs" in recommended.stdout
    assert "guía general" in recommended.stdout
    assert preview.exit_code == 0, preview.stdout
    assert "Repite sin --dry-run" in preview.stdout
    assert not (project / ".ai").exists()

    enabled = runner.invoke(app, ["skill", "enable", "nextjs", str(project)])
    sync_preview = runner.invoke(app, ["skill", "sync", str(project), "--dry-run"])

    assert enabled.exit_code == 0, enabled.stdout
    assert sync_preview.exit_code == 0, sync_preview.stdout
    assert not (project / ".ai" / "SKILLS.md").exists()

    synced = runner.invoke(app, ["skill", "sync", str(project)])

    assert synced.exit_code == 0, synced.stdout
    assert "Next.js" in (project / ".ai" / "SKILLS.md").read_text(encoding="utf-8")
    assert "Local project" not in (project / "AGENTS.md").read_text(encoding="utf-8")
    assert global_home.exists()

def test_pack_cli_reports_reason_and_applies_pack_additively(tmp_path: Path, monkeypatch):
    project = tmp_path / "typescript-project"
    project.mkdir()
    (project / "package.json").write_text(
        '{"dependencies":{"@nestjs/core":"^10"}}',
        encoding="utf-8",
    )
    (project / "main.ts").write_text("export const main = true\n", encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    first = runner.invoke(app, ["skill", "enable", "architecture", str(project)])
    detected = runner.invoke(app, ["pack", "detect", str(project)])
    preview = runner.invoke(app, ["pack", "apply", "nestjs-api", str(project), "--dry-run"])
    applied = runner.invoke(app, ["pack", "apply", "nestjs-api", str(project)])

    assert first.exit_code == 0, first.stdout
    assert detected.exit_code == 0, detected.stdout
    assert "nestjs-api" in detected.stdout
    assert "stack detectado: nestjs" in detected.stdout
    assert preview.exit_code == 0, preview.stdout
    assert "se añadirían" in preview.stdout
    assert applied.exit_code == 0, applied.stdout
    assert "architecture" in (project / ".ai" / "chxchx-skills.toml").read_text(encoding="utf-8")
    assert "nestjs" in (project / ".ai" / "chxchx-skills.toml").read_text(encoding="utf-8")

def test_pack_apply_detected_applies_union_without_pack_names(tmp_path: Path, monkeypatch):
    project = tmp_path / "fullstack"
    project.mkdir()
    (project / "package.json").write_text(
        '{"dependencies":{"next":"^15","react":"^19","@nestjs/core":"^10"}}',
        encoding="utf-8",
    )
    (project / "app.tsx").write_text("export const App = () => null\n", encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    preview = runner.invoke(app, ["pack", "apply-detected", str(project), "--dry-run"])

    assert preview.exit_code == 0, preview.stdout
    assert "Packs detectados:" in preview.stdout
    assert not (project / ".ai").exists()

    applied = runner.invoke(app, ["pack", "apply-detected", str(project)])
    status = runner.invoke(app, ["skill", "status", str(project)])

    assert applied.exit_code == 0, applied.stdout
    assert "Skills únicas añadidas:" in applied.stdout
    assert status.exit_code == 0, status.stdout
    assert "skills habilitadas" in status.stdout
    assert "nextjs" in (project / ".ai" / "chxchx-skills.toml").read_text(encoding="utf-8")


def test_skill_and_pack_lists_report_catalog_size():
    skills = runner.invoke(app, ["skill", "list"])
    packs = runner.invoke(app, ["pack", "list"])

    assert skills.exit_code == 0, skills.stdout
    assert "17 skills disponibles" in skills.stdout
    assert packs.exit_code == 0, packs.stdout
    assert "11 packs" in packs.stdout
    assert "nestjs-api" in packs.stdout
    assert "nestjs" in packs.stdout
    assert "typescript-engineering" in packs.stdout
