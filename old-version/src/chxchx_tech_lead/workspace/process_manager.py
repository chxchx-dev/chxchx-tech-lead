from __future__ import annotations

import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Sequence

from ..core.paths import home_dir
from ..core.trust import is_trusted
from .models import ProcessConfig, WorkspaceStatus
from .process_models import ManagedProcess, ProcessActionResult, ProcessStatus
from .process_runtime import command_available as _command_available
from .process_runtime import pid_alive as _pid_alive
from .process_runtime import pid_matches_command as _pid_matches_command
from .process_runtime import terminate_posix_process_group as _terminate_posix_process_group
from .process_runtime import terminate_windows_process_tree as _terminate_windows_process_tree
from .state import WorkspaceState, load_state, save_state


class ProcessManagerError(RuntimeError):
    """Error accionable al administrar un proceso del workspace."""

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
        if record.status is ProcessStatus.UNKNOWN and record.pid is not None:
            self._persist()
            raise ProcessManagerError(
                f"No detuve `{process_id}`: el PID {record.pid} no coincide con el comando administrado"
            )
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
                _terminate_windows_process_tree(
                    record.pid,
                    handle,
                    timeout_seconds=_PROCESS_STOP_TIMEOUT_SECONDS,
                )
            else:
                _terminate_posix_process_group(
                    record.pid,
                    handle,
                    timeout_seconds=_PROCESS_STOP_TIMEOUT_SECONDS,
                )
        except (OSError, subprocess.TimeoutExpired) as exc:
            record.status = ProcessStatus.UNKNOWN
            self._persist()
            raise ProcessManagerError(f"No pude detener `{process_id}`: {exc}") from exc
        record.status = ProcessStatus.EXITED
        record.exit_code = handle.returncode if handle is not None else 0
        self._handles.pop(process_id, None)
        self._persist()
        return ProcessActionResult(record, True, f"Detenido `{process_id}`")

    def read_log(self, process_id: str, *, max_bytes: int = 16_000) -> str:
        if process_id not in self.configs:
            raise ProcessManagerError(f"No existe el proceso configurado: {process_id}")
        record = self._records.get(process_id)
        log_path = record.log_path if record and record.log_path else _log_path(
            self.project_root, process_id
        )
        try:
            with log_path.open("rb") as log_file:
                log_file.seek(0, os.SEEK_END)
                size = log_file.tell()
                log_file.seek(max(0, size - max_bytes))
                content = log_file.read(max_bytes)
        except FileNotFoundError:
            return ""
        except OSError as exc:
            raise ProcessManagerError(f"No pude leer el log de `{process_id}`: {exc}") from exc
        return "\n".join(content.decode("utf-8", errors="replace").splitlines()[-300:])

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
        elif _pid_alive(record.pid):
            record.status = (
                ProcessStatus.RUNNING
                if self._pid_matches(record.pid, record.command, record.shell)
                else ProcessStatus.UNKNOWN
            )
        elif record.status in {ProcessStatus.RUNNING, ProcessStatus.UNKNOWN}:
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
