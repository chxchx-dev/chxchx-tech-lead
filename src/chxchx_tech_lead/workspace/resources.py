from __future__ import annotations

import os
import shutil
import subprocess
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Callable, Iterable

from .models import ResourceConfig
from .process_manager import ManagedProcess, ProcessStatus


class ResourceSeverity(StrEnum):
    OK = "OK"
    WARNING = "WARNING"
    CRITICAL = "CRITICAL"
    UNKNOWN = "UNKNOWN"


@dataclass(frozen=True, slots=True)
class SystemResources:
    total_bytes: int
    used_bytes: int
    available_bytes: int
    swap_total_bytes: int = 0
    swap_used_bytes: int = 0
    cpu_percent: float | None = None

    @property
    def memory_percent(self) -> float:
        if self.total_bytes <= 0:
            return 0.0
        return self.used_bytes / self.total_bytes * 100

    @property
    def swap_percent(self) -> float:
        if self.swap_total_bytes <= 0:
            return 0.0
        return self.swap_used_bytes / self.swap_total_bytes * 100


@dataclass(frozen=True, slots=True)
class ProcessResources:
    process_id: str
    label: str
    pid: int | None
    rss_bytes: int = 0
    cpu_percent: float | None = None
    status: ProcessStatus = ProcessStatus.STOPPED


@dataclass(frozen=True, slots=True)
class ProcessResourceSummary:
    running_count: int
    rss_bytes: int
    cpu_percent: float | None
    measured_cpu_count: int

    def memory_percent(self, total_memory_bytes: int) -> float | None:
        if total_memory_bytes <= 0:
            return None
        return self.rss_bytes / total_memory_bytes * 100


SystemReader = Callable[[], SystemResources]
ProcessReader = Callable[[int], tuple[int, float | None] | None]


class ResourceManager:
    """Mide recursos y solo informa umbrales; nunca mata procesos automáticamente."""

    def __init__(
        self,
        config: ResourceConfig | None = None,
        *,
        system_reader: SystemReader | None = None,
        process_reader: ProcessReader | None = None,
    ):
        self.config = config or ResourceConfig()
        self._system_reader = system_reader or read_system_resources
        self._process_reader = process_reader or read_process_resources

    def system(self) -> SystemResources:
        return self._system_reader()

    def severity(self, resources: SystemResources | None = None) -> ResourceSeverity:
        snapshot = resources or self.system()
        if snapshot.total_bytes <= 0:
            return ResourceSeverity.UNKNOWN
        if snapshot.memory_percent >= self.config.critical_memory_percent:
            return ResourceSeverity.CRITICAL
        if (
            snapshot.memory_percent >= self.config.warn_memory_percent
            or snapshot.swap_percent >= self.config.warn_swap_percent
        ):
            return ResourceSeverity.WARNING
        return ResourceSeverity.OK

    def processes(self, managed: Iterable[ManagedProcess]) -> list[ProcessResources]:
        result = []
        for process in managed:
            rss = 0
            cpu = None
            if process.pid is not None and process.status is ProcessStatus.RUNNING:
                measurement = self._process_reader(process.pid)
                if measurement is not None:
                    rss, cpu = measurement
            result.append(ProcessResources(process.id, process.label, process.pid, rss, cpu, process.status))
        return result


def summarize_process_resources(processes: Iterable[ProcessResources]) -> ProcessResourceSummary:
    running = [
        process
        for process in processes
        if process.status is ProcessStatus.RUNNING and process.pid is not None
    ]
    measured_cpu = [process.cpu_percent for process in running if process.cpu_percent is not None]
    return ProcessResourceSummary(
        running_count=len(running),
        rss_bytes=sum(process.rss_bytes for process in running),
        cpu_percent=sum(measured_cpu) if measured_cpu else None,
        measured_cpu_count=len(measured_cpu),
    )


def read_system_resources() -> SystemResources:
    """Usa psutil si está disponible y deja un fallback Linux sin dependencia extra."""
    try:
        import psutil  # type: ignore
    except ImportError:
        return _read_proc_resources()
    memory = psutil.virtual_memory()
    swap = psutil.swap_memory()
    return SystemResources(
        total_bytes=memory.total,
        used_bytes=memory.used,
        available_bytes=memory.available,
        swap_total_bytes=swap.total,
        swap_used_bytes=swap.used,
        cpu_percent=psutil.cpu_percent(interval=None),
    )


def read_process_resources(pid: int) -> tuple[int, float | None] | None:
    try:
        import psutil  # type: ignore
    except ImportError:
        return _read_proc_process(pid)
    try:
        process = psutil.Process(pid)
        return process.memory_info().rss, process.cpu_percent(interval=None)
    except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
        return None


def _read_proc_resources() -> SystemResources:
    values: dict[str, int] = {}
    meminfo = Path("/proc/meminfo")
    if not meminfo.exists():
        return SystemResources(0, 0, 0, cpu_percent=_fallback_cpu_percent())
    try:
        for line in meminfo.read_text(encoding="utf-8").splitlines():
            key, _, raw = line.partition(":")
            parts = raw.strip().split()
            if parts and parts[0].isdigit():
                values[key] = int(parts[0]) * (1024 if len(parts) > 1 and parts[1] == "kB" else 1)
    except OSError:
        return SystemResources(0, 0, 0, cpu_percent=_fallback_cpu_percent())
    total = values.get("MemTotal", 0)
    available = values.get("MemAvailable", values.get("MemFree", 0))
    swap_total = values.get("SwapTotal", 0)
    swap_free = values.get("SwapFree", 0)
    return SystemResources(
        total_bytes=total,
        used_bytes=max(total - available, 0),
        available_bytes=available,
        swap_total_bytes=swap_total,
        swap_used_bytes=max(swap_total - swap_free, 0),
        cpu_percent=_fallback_cpu_percent(),
    )


def _read_proc_process(pid: int) -> tuple[int, float | None] | None:
    statm = Path(f"/proc/{pid}/statm")
    if not statm.exists():
        return None
    try:
        resident_pages = int(statm.read_text(encoding="utf-8").split()[1])
    except (OSError, IndexError, ValueError):
        return None
    return resident_pages * os.sysconf("SC_PAGE_SIZE"), None


def _fallback_cpu_percent() -> float | None:
    if not hasattr(os, "getloadavg") or not os.cpu_count():
        return None
    try:
        return min(max(os.getloadavg()[0] / os.cpu_count() * 100, 0.0), 100.0)
    except OSError:
        return None


def format_bytes(value: int) -> str:
    if value < 0:
        return "N/D"
    units = ("B", "KB", "MB", "GB", "TB")
    amount = float(value)
    for unit in units:
        if amount < 1024 or unit == units[-1]:
            return f"{amount:.1f} {unit}"
        amount /= 1024
    return "N/D"
