from pathlib import Path

import pytest

from chxchx_tech_lead.workspace import state as state_module
from chxchx_tech_lead.workspace.models import WorkspaceStatus
from chxchx_tech_lead.workspace.state import load_state, save_state, set_workspace_status


def test_workspace_state_is_read_only_until_saved(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()

    state = load_state(project)
    assert state.status is WorkspaceStatus.STOPPED
    assert not (tmp_path / "global").exists()

    state.status = WorkspaceStatus.ACTIVE
    state.session_name = "demo"
    state.process_ids = ["api"]
    save_state(state)

    loaded = load_state(project)
    assert loaded.status is WorkspaceStatus.ACTIVE
    assert loaded.session_name == "demo"
    assert loaded.process_ids == ["api"]


def test_workspace_status_can_be_suspended_and_resumed(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()

    set_workspace_status(project, WorkspaceStatus.SUSPENDED, session_name="demo")
    loaded = load_state(project)

    assert loaded.status is WorkspaceStatus.SUSPENDED
    assert loaded.session_name == "demo"


def test_save_state_keeps_previous_file_if_atomic_replace_fails(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()

    state = state_module.load_state(project)
    state.session_name = "initial"
    state_module.save_state(state)
    path = state_module.state_path()
    previous = path.read_text(encoding="utf-8")

    state.session_name = "updated"

    def fail_replace(source, destination):
        raise OSError("simulated interruption")

    monkeypatch.setattr(state_module.os, "replace", fail_replace)
    with pytest.raises(OSError, match="simulated interruption"):
        state_module.save_state(state)

    assert path.read_text(encoding="utf-8") == previous
    assert list(path.parent.glob(f".{path.name}.*.tmp")) == []
