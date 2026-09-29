from pathlib import Path
import signal
import sys

import pytest

from chxchx_tech_lead.workspace.models import ProcessConfig
from chxchx_tech_lead.workspace import process_manager as process_manager_module
from chxchx_tech_lead.workspace.process_manager import (
    _rotate_log,
    ManagedProcess,
    ProcessManager,
    ProcessManagerError,
    ProcessStatus,
)


def _config(command: list[str] | str, *, shell: bool = False) -> ProcessConfig:
    return ProcessConfig(id="worker", label="Worker", command=command, shell=shell)


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


def test_owned_process_handle_does_not_depend_on_platform_command_line_format(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(
        project,
        [_config([sys.executable, "-c", "import time; time.sleep(30)"])],
        trusted=True,
    )
    started = manager.start("worker")
    monkeypatch.setattr(
        manager,
        "_pid_matches",
        lambda *_args: pytest.fail("un handle propio ya identifica el proceso"),
    )

    stopped = manager.stop("worker")

    assert started.process.pid == stopped.process.pid
    assert stopped.process.status is ProcessStatus.EXITED


@pytest.mark.skipif(sys.platform == "win32", reason="la shell y la comprobación de PID difieren en Windows")
def test_process_manager_can_stop_an_explicit_shell_command(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(
        project,
        [_config("sleep 30", shell=True)],
        trusted=True,
    )

    started = manager.start("worker")
    stopped = manager.stop("worker")

    assert started.process.pid is not None
    assert stopped.process.status is ProcessStatus.EXITED


@pytest.mark.skipif(sys.platform == "win32", reason="los grupos de procesos POSIX no existen en Windows")
def test_process_manager_stops_the_isolated_process_group(tmp_path: Path, monkeypatch):
    import chxchx_tech_lead.workspace.process_manager as process_manager_module

    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(project, [_config([sys.executable, "-c", "pass"])], trusted=True)

    class FakeHandle:
        pid = 4242
        returncode = None

        def poll(self):
            return self.returncode

        def wait(self, timeout=None):
            self.returncode = 0
            return self.returncode

        def terminate(self):
            pytest.fail("debe señalizarse el grupo aislado, no solo el proceso padre")

    handle = FakeHandle()
    manager._records["worker"] = ManagedProcess(
        id="worker",
        label="Worker",
        command=[sys.executable, "-c", "pass"],
        cwd=project,
        status=ProcessStatus.RUNNING,
        pid=handle.pid,
    )
    manager._handles["worker"] = handle
    signals = []
    monkeypatch.setattr(manager, "_pid_matches", lambda *_args: True)
    monkeypatch.setattr(process_manager_module.os, "killpg", lambda pid, sig: signals.append((pid, sig)))
    monkeypatch.setattr(process_manager_module, "_process_group_alive", lambda _pid: False)

    result = manager.stop("worker")

    assert result.process.status is ProcessStatus.EXITED
    assert signals == [(handle.pid, signal.SIGTERM)]


@pytest.mark.skipif(sys.platform == "win32", reason="los grupos de procesos POSIX no existen en Windows")
def test_process_manager_escalates_only_after_graceful_group_stop_times_out(tmp_path: Path, monkeypatch):
    import chxchx_tech_lead.workspace.process_manager as process_manager_module

    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(project, [_config([sys.executable, "-c", "pass"])], trusted=True)

    class FakeHandle:
        pid = 4343
        returncode = None

        def poll(self):
            return self.returncode

        def wait(self, timeout=None):
            self.returncode = 0
            return self.returncode

        def terminate(self):
            pytest.fail("la terminación debe dirigirse al grupo aislado")

    handle = FakeHandle()
    manager._records["worker"] = ManagedProcess(
        id="worker",
        label="Worker",
        command=[sys.executable, "-c", "pass"],
        cwd=project,
        status=ProcessStatus.RUNNING,
        pid=handle.pid,
    )
    manager._handles["worker"] = handle
    signals = []
    monkeypatch.setattr(manager, "_pid_matches", lambda *_args: True)
    monkeypatch.setattr(process_manager_module.os, "killpg", lambda pid, sig: signals.append((pid, sig)))
    monkeypatch.setattr(process_manager_module, "_process_group_alive", lambda _pid: True)
    monkeypatch.setattr(process_manager_module, "_PROCESS_STOP_TIMEOUT_SECONDS", 0)

    manager.stop("worker")

    assert signals == [(handle.pid, signal.SIGTERM), (handle.pid, signal.SIGKILL)]


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


def test_process_manager_can_refresh_status_without_persisting(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    manager = ProcessManager(project, [_config([sys.executable, "-c", "pass"])], trusted=True)
    record = ManagedProcess(
        id="worker",
        label="Worker",
        command=[sys.executable, "-c", "pass"],
        cwd=project,
        status=ProcessStatus.RUNNING,
        pid=987654,
    )
    manager._records[record.id] = record

    def mark_exited(item):
        item.status = ProcessStatus.EXITED
        return item

    monkeypatch.setattr(manager, "_refresh", mark_exited)
    monkeypatch.setattr(manager, "_persist", lambda: pytest.fail("persist should not be called"))

    assert manager.list(persist=False)[0].status is ProcessStatus.EXITED


def test_process_logs_rotate_with_a_bounded_number_of_backups(tmp_path: Path, monkeypatch):
    monkeypatch.setattr(process_manager_module, "_PROCESS_LOG_MAX_BYTES", 8)
    monkeypatch.setattr(process_manager_module, "_PROCESS_LOG_BACKUPS", 2)
    log = tmp_path / "worker.log"

    log.write_bytes(b"first-log")
    _rotate_log(log)
    assert not log.exists()
    assert (tmp_path / "worker.log.1").read_bytes() == b"first-log"

    log.write_bytes(b"second-log")
    _rotate_log(log)
    assert (tmp_path / "worker.log.1").read_bytes() == b"second-log"
    assert (tmp_path / "worker.log.2").read_bytes() == b"first-log"

    log.write_bytes(b"third-log")
    _rotate_log(log)
    assert (tmp_path / "worker.log.1").read_bytes() == b"third-log"
    assert (tmp_path / "worker.log.2").read_bytes() == b"second-log"
    assert not (tmp_path / "worker.log.3").exists()


def test_process_manager_recovers_process_state_after_restart(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    command = [sys.executable, "-c", "import time; time.sleep(30)"]
    original = ProcessManager(project, [_config(command)], trusted=True)
    started = original.start("worker")

    recovered = ProcessManager(
        project,
        [_config(command)],
        trusted=True,
        pid_matches=lambda pid, expected, shell: True,
    )
    try:
        assert started.process.pid is not None
        assert recovered.list()[0].status is ProcessStatus.RUNNING

        stopped = recovered.stop("worker")
        assert stopped.process.status is ProcessStatus.EXITED
    finally:
        original.stop("worker")
