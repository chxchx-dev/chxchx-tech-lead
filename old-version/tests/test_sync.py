from pathlib import Path

from chxchx_tech_lead.core.sync import sync_project


def test_sync_is_idempotent_and_backed_up(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    first = sync_project(project)
    second = sync_project(project)

    assert first.changed is True
    assert first.backup is None
    assert second.changed is False
    assert second.backup is None
    assert (project / "AGENTS.md").exists()
    assert (project / "CLAUDE.md").exists()
