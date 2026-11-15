from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.paths import home_dir
from chxchx_tech_lead.core.registry import (
    last_project,
    load_registry,
    register_project,
    resolve_project_reference,
    set_last_project,
)


def test_register_dry_run_does_not_create_global_home(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))
    project = tmp_path / "project"
    project.mkdir()

    assert register_project(ProjectInfo(project, "project"), dry_run=True) is True
    assert not home_dir().exists()


def test_register_is_idempotent_and_keeps_timestamp(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))
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
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(global_home))
    (global_home / "projects.json").write_text("[]", encoding="utf-8")

    assert load_registry() == {"projects": []}


def test_registry_assigns_alias_and_resolves_it(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "My Project"
    project.mkdir()

    register_project(ProjectInfo(project, "My Project"))

    assert load_registry()["projects"][0]["alias"] == "my-project"
    assert resolve_project_reference("my-project") == project.resolve()
    assert resolve_project_reference("My Project") == project.resolve()


def test_last_project_is_persisted_idempotently(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()

    assert set_last_project(project) is True
    assert set_last_project(project) is False
    assert last_project() == project.resolve()
