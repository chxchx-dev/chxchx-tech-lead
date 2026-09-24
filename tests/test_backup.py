from pathlib import Path

from chichan_tech_lead.core.backup import backup_project, latest_backup, restore_backup


def test_restore_previous_backup_after_safety_backup(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHICHAN_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    target = project / "AGENTS.md"

    target.write_text("original\n", encoding="utf-8")
    previous = backup_project(project)
    target.write_text("changed\n", encoding="utf-8")
    restore_target = latest_backup(project)
    safety = backup_project(project)

    assert restore_target == previous
    assert safety != previous
    restore_backup(project, restore_target)
    assert target.read_text(encoding="utf-8") == "original\n"

def test_latest_backup_is_read_only_when_empty(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))
    project = tmp_path / "project"
    project.mkdir()

    assert latest_backup(project) is None
    assert not global_home.exists()

def test_restore_removes_managed_items_created_after_backup(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHICHAN_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()

    (project / "AGENTS.md").write_text("existing\n", encoding="utf-8")
    previous = backup_project(project)
    (project / ".ai").mkdir()
    (project / ".ai" / "PROJECT.md").write_text("created\n", encoding="utf-8")

    restore_backup(project, previous)
    assert not (project / ".ai").exists()
