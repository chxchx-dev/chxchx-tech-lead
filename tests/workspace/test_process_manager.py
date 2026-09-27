from pathlib import Path
import sys

import pytest

from chxchx_tech_lead.workspace.models import ProcessConfig
from chxchx_tech_lead.workspace.process_manager import ProcessManager, ProcessManagerError, ProcessStatus


def _config(command: list[str]) -> ProcessConfig:
    return ProcessConfig(id="worker", label="Worker", command=command)


def test_process_manager_starts_does_not_duplicate_and_stops_process(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(
        project,
        [_config([sys.executable, "-c", "import time; time.sleep(30)"])],
        trusted=True,
    )

    started = manager.start("worker")
    duplicate = manager.start("worker")

    assert started.process.status is ProcessStatus.RUNNING
    assert started.process.pid is not None
    assert duplicate.changed is False
    assert "ya está ejecutándose" in duplicate.message

    stopped = manager.stop("worker")

    assert stopped.changed is True
    assert stopped.process.status is ProcessStatus.EXITED
    assert manager.list()[0].status is ProcessStatus.EXITED


def test_process_manager_requires_trust_and_valid_cwd(tmp_path: Path):
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(project, [_config([sys.executable, "-c", "pass"])], trusted=False)

    with pytest.raises(ProcessManagerError, match="no es confiable"):
        manager.start("worker")


def test_process_manager_dry_run_never_starts_or_writes_state(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(
        project,
        [_config([sys.executable, "-c", "import time; time.sleep(30)"])],
        trusted=True,
    )

    result = manager.start("worker", dry_run=True)

    assert result.changed is True
    assert result.process.status is ProcessStatus.RUNNING
    assert not (tmp_path / "global").exists()


def test_process_manager_rejects_missing_executable(tmp_path: Path):
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(project, [_config(["definitely-not-installed-chxchx-tech", "run"])], trusted=True)

    with pytest.raises(ProcessManagerError, match="No encuentro el ejecutable"):
        manager.start("worker")
