from pathlib import Path

from chxchx_tech_lead.workspace.memory_history import list_memory_notes


def _write_note(root: Path, name: str, content: str) -> Path:
    path = root / ".ai" / "memory" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def test_lists_only_notes_from_selected_project(tmp_path: Path):
    bokana = tmp_path / "bokana"
    security = tmp_path / "security"
    _write_note(bokana, "api.md", "# API decisions\nBokana-only context.")
    _write_note(security, "security.md", "# Security review\nSecurity-only context.")

    notes = list_memory_notes(bokana)

    assert [note.title for note in notes] == ["API decisions"]
    assert "Bokana-only" in notes[0].content
    assert notes[0].path.is_relative_to(bokana.resolve())


def test_filters_notes_by_title_or_body_case_insensitively(tmp_path: Path):
    project = tmp_path / "project"
    _write_note(project, "architecture.md", "# Architecture\nUses SQLite for persistence.")
    _write_note(project, "handoff.md", "# Handoff\nNext: validate the CLI.")

    notes = list_memory_notes(project, query="sqlite")

    assert len(notes) == 1
    assert notes[0].title == "Architecture"
    assert "SQLite" in notes[0].content


def test_empty_or_missing_project_memory_returns_no_notes(tmp_path: Path):
    assert list_memory_notes(tmp_path / "missing") == []

    project = tmp_path / "project"
    (project / ".ai" / "memory").mkdir(parents=True)
    assert list_memory_notes(project) == []
