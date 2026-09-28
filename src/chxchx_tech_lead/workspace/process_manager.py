from __future__ import annotations

import os
import shutil
import signal
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from enum import StrEnum
from pathlib import Path
from typing import Any, Callable, Sequence

from ..core.paths import ensure_home, home_dir
from ..core.trust import is_trusted
from .models import ProcessConfig, WorkspaceStatus
from .state import WorkspaceState, load_state, save_state


class ProcessStatus(StrEnum):
    RUNNING = "RUNNING"
    STOPPED = "STOPPED"
    EXITED = "EXITED"
    FAILED = "FAILED"
    UNKNOWN = "UNKNOWN"


class ProcessManagerError(RuntimeError):
    """Error accionable al administrar un proceso del workspace."""


@dataclass(slots=True)
class ManagedProcess:
    id: str
    label: str
    command: list[str] | str
    cwd: Path
    status: ProcessStatus = ProcessStatus.STOPPED
    pid: int | None = None
    port: int | None = None
    started_at: datetime | None = None
    exit_code: int | None = None
    log_path: Path | None = None
    shell: bool = False

    def to_mapping(self) -> dict[str, Any]:
        data = asdict(self)
        data["cwd"] = str(self.cwd)
        data["status"] = self.status.value
        data["started_at"] = self.started_at.isoformat() if self.started_at else None
        data["log_path"] = str(self.log_path) if self.log_path else None
        return data

    @classmethod
    def from_mapping(cls, raw: dict[str, Any]) -> "ManagedProcess | None":
        try:
            status = ProcessStatus(raw.get("status", ProcessStatus.STOPPED))
        except ValueError:
            status = ProcessStatus.UNKNOWN
        command = raw.get("command")
        if not isinstance(command, (str, list)) or (isinstance(command, list) and not all(isinstance(item, str) for item in command)):
            return None
        if not isinstance(raw.get("id"), str) or not isinstance(raw.get("label"), str) or not isinstance(raw.get("cwd"), str):
            return None
        started_at = None
        if isinstance(raw.get("started_at"), str):
            try:
                started_at = datetime.fromisoformat(raw["started_at"])
            except ValueError:
                pass
        return cls(
            id=raw["id"],
            label=raw["label"],
            command=list(command) if isinstance(command, list) else command,
            cwd=Path(raw["cwd"]),
            status=status,
            pid=raw.get("pid") if isinstance(raw.get("pid"), int) else None,
            port=raw.get("port") if isinstance(raw.get("port"), int) else None,
            started_at=started_at,
            exit_code=raw.get("exit_code") if isinstance(raw.get("exit_code"), int) else None,
            log_path=Path(raw["log_path"]) if isinstance(raw.get("log_path"), str) else None,
            shell=raw.get("shell") is True,
        )


@dataclass(frozen=True, slots=True)
class ProcessActionResult:
    process: ManagedProcess
    changed: bool
    message: str


PopenFactory = Callable[..., subprocess.Popen[Any]]
_PROCESS_STOP_TIMEOUT_SECONDS = 5
_PROCESS_LOG_MAX_BYTES = 5 * 1024 * 1024
_PROCESS_LOG_BACKUPS = 3


class ProcessManager:
    """Inicia y detiene procesos configurados sin delegar en un shell implícito."""

    def __init__(
        self,
        project_root: Path,
        configs: Sequence[ProcessConfig],
        *,
        trusted: bool | None = None,
        popen_factory: PopenFactory = subprocess.Popen,
        pid_matches: Callable[[int, list[str] | str, bool], bool] | None = None,
    ):
        self.project_root = project_root.resolve()
        self.configs = {config.id: config for config in configs}
        self.trusted = is_trusted(self.project_root) if trusted is None else trusted
        self._popen = popen_factory
        self._pid_matches = pid_matches or _pid_matches_command
        self._handles: dict[str, subprocess.Popen[Any]] = {}
        state = load_state(self.project_root)
        self._records = {
            key: process
            for key, raw in state.processes.items()
            if (process := ManagedProcess.from_mapping(raw)) is not None
        }

    def list(self, *, persist: bool = True) -> list[ManagedProcess]:
        before = {key: record.to_mapping() for key, record in self._records.items()}
        records = [self._refresh(record) for record in self._records.values()]
        after = {key: record.to_mapping() for key, record in self._records.items()}
        if persist and before != after:
            self._persist()
        return sorted(records, key=lambda item: item.id)

    def start(self, process_id: str, dry_run: bool = False) -> ProcessActionResult:
        config = self.configs.get(process_id)
        if config is None:
            raise ProcessManagerError(f"No existe el proceso configurado: {process_id}")
        if not self.trusted:
            raise ProcessManagerError("El proyecto no es confiable; marca la ruta como trusted antes de ejecutar procesos")
        cwd = (self.project_root / config.cwd).resolve()
        if self.project_root not in (cwd, *cwd.parents) or not cwd.is_dir():
            raise ProcessManagerError(f"No existe el cwd de `{process_id}`: {cwd}")
        if not _command_available(config.command, cwd, config.shell):
            executable = config.command if isinstance(config.command, str) else config.command[0]
            raise ProcessManagerError(f"No encuentro el ejecutable de `{process_id}`: {executable}")

        current = self._records.get(process_id)
        if current is not None:
            self._refresh(current)
            if current.status is ProcessStatus.RUNNING:
                return ProcessActionResult(current, False, f"`{process_id}` ya está ejecutándose (PID {current.pid})")

        log_path = _log_path(self.project_root, process_id)
        command = config.command
        normalized = list(command) if isinstance(command, list) else command
        record = ManagedProcess(
            id=config.id,
            label=config.label,
            command=normalized,
            cwd=cwd,
            status=ProcessStatus.STOPPED,
            port=config.port,
            log_path=log_path,
            shell=config.shell,
        )
        if dry_run:
            record.status = ProcessStatus.RUNNING
            record.started_at = datetime.now(timezone.utc)
            return ProcessActionResult(record, True, f"DRY RUN: iniciar `{process_id}`")

        log_path.parent.mkdir(parents=True, exist_ok=True)
        _rotate_log(log_path)
        log_file = log_path.open("ab")
        try:
            process = self._popen(
                command,
                cwd=str(cwd),
                shell=config.shell,
                stdout=log_file,
                stderr=subprocess.STDOUT,
                start_new_session=(os.name != "nt"),
            )
        except (OSError, ValueError) as exc:
            log_file.close()
            record.status = ProcessStatus.FAILED
            self._records[process_id] = record
            self._persist()
            raise ProcessManagerError(f"No pude iniciar `{process_id}`: {exc}") from exc
        finally:
            log_file.close()

        record.pid = process.pid
        record.status = ProcessStatus.RUNNING
        record.started_at = datetime.now(timezone.utc)
        self._handles[process_id] = process
        self._records[process_id] = record
        self._persist()
        return ProcessActionResult(record, True, f"Iniciado `{process_id}` (PID {process.pid})")

    def stop(self, process_id: str, dry_run: bool = False) -> ProcessActionResult:
        record = self._records.get(process_id)
        if record is None:
            raise ProcessManagerError(f"No hay estado para el proceso: {process_id}")
        self._refresh(record)
        if record.status is not ProcessStatus.RUNNING or record.pid is None:
            return ProcessActionResult(record, False, f"`{process_id}` ya está detenido")
        if dry_run:
            return ProcessActionResult(record, True, f"DRY RUN: detener `{process_id}` (PID {record.pid})")
        handle = self._handles.get(process_id)
        # A retained Popen handle identifies the exact child created by this
        # manager. Re-parsing `ps` output here is both redundant and brittle
        # across platforms (macOS may display a different executable path).
        # Recovered processes have no handle and still require PID validation.
        if handle is None and not self._pid_matches(record.pid, record.command, record.shell):
            record.status = ProcessStatus.UNKNOWN
            self._persist()
            raise ProcessManagerError(
                f"No detuve `{process_id}`: el PID {record.pid} ya no coincide con el comando administrado"
            )

        try:
            if os.name == "nt":
                if handle is not None:
                    handle.terminate()
                    try:
                        handle.wait(timeout=_PROCESS_STOP_TIMEOUT_SECONDS)
                    except subprocess.TimeoutExpired:
                        subprocess.run(
                            ["taskkill", "/PID", str(record.pid), "/T", "/F"],
                            check=False,
                            capture_output=True,
                        )
                        handle.wait(timeout=_PROCESS_STOP_TIMEOUT_SECONDS)
                else:
                    subprocess.run(
                        ["taskkill", "/PID", str(record.pid), "/T", "/F"],
                        check=False,
                        capture_output=True,
                    )
            else:
                _terminate_posix_process_group(record.pid, handle)
        except (OSError, subprocess.TimeoutExpired) as exc:
            record.status = ProcessStatus.UNKNOWN
            self._persist()
            raise ProcessManagerError(f"No pude detener `{process_id}`: {exc}") from exc
        record.status = ProcessStatus.EXITED
        record.exit_code = handle.returncode if handle is not None else 0
        self._handles.pop(process_id, None)
        self._persist()
        return ProcessActionResult(record, True, f"Detenido `{process_id}`")

    def _refresh(self, record: ManagedProcess) -> ManagedProcess:
        handle = self._handles.get(record.id)
        if handle is not None:
            code = handle.poll()
            if code is None:
                record.status = ProcessStatus.RUNNING
            else:
                record.status = ProcessStatus.EXITED if code == 0 else ProcessStatus.FAILED
                record.exit_code = code
                self._handles.pop(record.id, None)
            return record
        if record.pid is None:
            record.status = ProcessStatus.STOPPED
        elif _pid_alive(record.pid) and self._pid_matches(record.pid, record.command, record.shell):
            record.status = ProcessStatus.RUNNING
        elif record.status is ProcessStatus.RUNNING:
            record.status = ProcessStatus.EXITED
        return record

    def _persist(self) -> None:
        state = load_state(self.project_root)
        records = {key: record.to_mapping() for key, record in self._records.items()}
        state.processes = records
        state.process_ids = [record.id for record in self._records.values() if record.status is ProcessStatus.RUNNING]
        state.status = WorkspaceStatus.ACTIVE if state.process_ids else WorkspaceStatus.STOPPED
        save_state(state)


def _log_path(project_root: Path, process_id: str) -> Path:
    safe_name = "".join(char if char.isalnum() or char in "-_." else "-" for char in project_root.name)
    return home_dir() / "logs" / "workspaces" / f"{safe_name}-{process_id}.log"


def _rotate_log(path: Path) -> None:
    if not path.exists() or path.stat().st_size < _PROCESS_LOG_MAX_BYTES:
        return

    oldest = Path(f"{path}.{_PROCESS_LOG_BACKUPS}")
    oldest.unlink(missing_ok=True)
    for index in range(_PROCESS_LOG_BACKUPS - 1, 0, -1):
        source = Path(f"{path}.{index}")
        if source.exists():
            source.replace(Path(f"{path}.{index + 1}"))
    path.replace(Path(f"{path}.1"))


def _command_available(command: list[str] | str, cwd: Path, shell: bool) -> bool:
    if shell:
        return isinstance(command, str) and bool(command.strip())
    if not isinstance(command, list) or not command:
        return False
    executable = command[0]
    if "/" in executable or "\\" in executable:
        return (cwd / executable).exists() if not Path(executable).is_absolute() else Path(executable).exists()
    return shutil.which(executable) is not None


def _pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except (OSError, ProcessLookupError):
        return False
    return True


def _process_group_alive(process_group_id: int) -> bool:
    try:
        os.killpg(process_group_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _signal_process_group(process_group_id: int, process_signal: int) -> bool:
    try:
        os.killpg(process_group_id, process_signal)
    except ProcessLookupError:
        return False
    return True


def _terminate_posix_process_group(
    process_group_id: int,
    handle: subprocess.Popen[Any] | None,
) -> None:
    """Stop only the isolated process group created for this managed process."""
    if not _signal_process_group(process_group_id, signal.SIGTERM):
        return

    if handle is not None:
        try:
            handle.wait(timeout=_PROCESS_STOP_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            _signal_process_group(process_group_id, signal.SIGKILL)
            handle.wait(timeout=_PROCESS_STOP_TIMEOUT_SECONDS)
            return

    deadline = time.monotonic() + _PROCESS_STOP_TIMEOUT_SECONDS
    while _process_group_alive(process_group_id) and time.monotonic() < deadline:
        time.sleep(0.05)
    if _process_group_alive(process_group_id):
        _signal_process_group(process_group_id, signal.SIGKILL)
        if handle is not None and handle.poll() is None:
            handle.wait(timeout=_PROCESS_STOP_TIMEOUT_SECONDS)


def _pid_matches_command(pid: int, command: list[str] | str, shell: bool) -> bool:
    if not _pid_alive(pid):
        return False
    if shell:
        shell_executable = os.environ.get("COMSPEC", "cmd.exe") if os.name == "nt" else "sh"
        expected_name = Path(shell_executable).name
    else:
        expected = command if isinstance(command, str) else command[0]
        expected_name = Path(expected.split()[0]).name
    if sys.platform.startswith("linux"):
        try:
            raw = Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace").strip()
            return bool(raw) and Path(raw.split()[0]).name == expected_name
        except OSError:
            return False
    if os.name != "nt":
        result = subprocess.run(["ps", "-p", str(pid), "-o", "command="], capture_output=True, text=True, check=False)
        raw = result.stdout.strip()
        return result.returncode == 0 and bool(raw) and Path(raw.split()[0]).name == expected_name
    result = subprocess.run(
        ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0 and expected_name.lower() in result.stdout.lower()
