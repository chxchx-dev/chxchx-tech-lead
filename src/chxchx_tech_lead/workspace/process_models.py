from __future__ import annotations

from dataclasses import asdict, dataclass
from datetime import datetime
from enum import StrEnum
from pathlib import Path
from typing import Any


class ProcessStatus(StrEnum):
    RUNNING = "RUNNING"
    STOPPED = "STOPPED"
    EXITED = "EXITED"
    FAILED = "FAILED"
    UNKNOWN = "UNKNOWN"


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
    def from_mapping(cls, raw: dict[str, Any]) -> ManagedProcess | None:
        try:
            status = ProcessStatus(raw.get("status", ProcessStatus.STOPPED))
        except ValueError:
            status = ProcessStatus.UNKNOWN
        command = raw.get("command")
        if not isinstance(command, (str, list)) or (
            isinstance(command, list) and not all(isinstance(item, str) for item in command)
        ):
            return None
        if not all(isinstance(raw.get(key), str) for key in ("id", "label", "cwd")):
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
