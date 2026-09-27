from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.workspace.manager import WorkspaceManager
from chxchx_tech_lead.workspace.models import WorkspaceStatus


def test_manager_inspects_config_without_running_processes(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    info = ProjectInfo(project, "project")

    inspection = WorkspaceManager(info).inspect()

    assert inspection.error is None
    assert inspection.config is not None
    assert inspection.config.name == "project"
    assert inspection.state.status is WorkspaceStatus.STOPPED
    assert inspection.trusted is False
    assert not (tmp_path / "global").exists()
