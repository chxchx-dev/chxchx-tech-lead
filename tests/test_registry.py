from pathlib import Path

from chichan_tech_lead.core.models import ProjectInfo
from chichan_tech_lead.core.paths import home_dir
from chichan_tech_lead.core.registry import load_registry, register_project


def test_register_dry_run_does_not_create_global_home(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))
    project = tmp_path / "project"
    project.mkdir()

    assert register_project(ProjectInfo(project, "project"), dry_run=True) is True
    assert not home_dir().exists()


def test_register_is_idempotent_and_keeps_timestamp(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))
    project = tmp_path / "project"
    project.mkdir()
    info = ProjectInfo(project, "project")

    assert register_project(info) is True
    first = load_registry()["projects"][0]["updated_at"]
    assert register_project(info) is False
    assert load_registry()["projects"][0]["updated_at"] == first


def test_load_registry_recovers_from_invalid_shape(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    global_home.mkdir()
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))
    (global_home / "projects.json").write_text("[]", encoding="utf-8")

    assert load_registry() == {"projects": []}
