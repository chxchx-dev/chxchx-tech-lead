from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.mcp_diagnostics import McpDiagnostic
from chxchx_tech_lead.workspace.autosave_status import diagnose_autosave, format_autosave_status


def test_autosave_status_reports_ready_setup_and_latest_project_note(tmp_path: Path, monkeypatch):
    project_root = tmp_path / "project"
    project_root.mkdir()
    (project_root / "AGENTS.md").write_text("Use write_memory to checkpoint.", encoding="utf-8")
    (project_root / "CLAUDE.md").write_text("Use write_memory to checkpoint.", encoding="utf-8")
    memory = project_root / ".ai" / "memory"
    memory.mkdir(parents=True)
    (memory / "decision.md").write_text("# Architecture decision\nUse SQLite.", encoding="utf-8")
    monkeypatch.setattr(
        "chxchx_tech_lead.workspace.autosave_status.diagnose_project_mcp",
        lambda _project: [
            McpDiagnostic("Codex", "basic-memory", "OK", "proyecto", "configured"),
            McpDiagnostic("Claude Code", "basic-memory", "OK", "local", "configured"),
        ],
    )

    status = diagnose_autosave(ProjectInfo(project_root, "project"))

    assert status.ready
    assert status.latest_note is not None
    assert status.latest_note.title == "Architecture decision"
    display = format_autosave_status(status)
    assert "Guardado de contexto · CONFIGURACIÓN LISTA" in display
    assert "Última nota observada: Architecture decision" in display


def test_autosave_status_warns_when_instructions_or_mcp_are_missing(tmp_path: Path, monkeypatch):
    project_root = tmp_path / "project"
    project_root.mkdir()
    (project_root / "AGENTS.md").write_text("No checkpoint rule", encoding="utf-8")
    (project_root / "CLAUDE.md").write_text("No checkpoint rule", encoding="utf-8")
    monkeypatch.setattr(
        "chxchx_tech_lead.workspace.autosave_status.diagnose_project_mcp",
        lambda _project: [
            McpDiagnostic("Codex", "basic-memory", "OK", "proyecto", "configured"),
            McpDiagnostic("Claude Code", "basic-memory", "FALTA", "-", "missing"),
        ],
    )

    status = diagnose_autosave(ProjectInfo(project_root, "project"))

    assert not status.ready
    display = format_autosave_status(status)
    assert "REVISAR CONFIGURACIÓN" in display
    assert "Última nota: todavía no hay notas persistidas" in display
    assert "integrate --dry-run --refresh --client claude" in display
