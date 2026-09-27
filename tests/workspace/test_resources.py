from pathlib import Path

from chxchx_tech_lead.workspace.models import ProcessConfig, ResourceConfig
from chxchx_tech_lead.workspace.process_manager import ManagedProcess, ProcessStatus
from chxchx_tech_lead.workspace.resources import (
    ProcessResources,
    ResourceManager,
    ResourceSeverity,
    SystemResources,
    format_bytes,
)


def test_resource_manager_classifies_memory_and_swap_thresholds():
    config = ResourceConfig(warn_memory_percent=75, critical_memory_percent=90, warn_swap_percent=40)
    manager = ResourceManager(config, system_reader=lambda: SystemResources(100, 80, 20, 100, 10, 25))

    snapshot = manager.system()

    assert manager.severity(snapshot) is ResourceSeverity.WARNING
    critical = SystemResources(100, 70, 30, 100, 95, 25)
    assert manager.severity(critical) is ResourceSeverity.WARNING
    assert manager.severity(SystemResources(100, 95, 5)) is ResourceSeverity.CRITICAL


def test_resource_manager_reads_metrics_only_for_running_processes(tmp_path: Path):
    seen = []
    manager = ResourceManager(
        process_reader=lambda pid: seen.append(pid) or (12_345, 4.5),
    )
    running = ManagedProcess("api", "API", ["python"], tmp_path, ProcessStatus.RUNNING, pid=123)
    stopped = ManagedProcess("worker", "Worker", ["python"], tmp_path, ProcessStatus.STOPPED, pid=456)

    metrics = manager.processes([running, stopped])

    assert seen == [123]
    assert metrics == [
        ProcessResources("api", "API", 123, 12_345, 4.5, ProcessStatus.RUNNING),
        ProcessResources("worker", "Worker", 456, 0, None, ProcessStatus.STOPPED),
    ]


def test_format_bytes_is_human_readable():
    assert format_bytes(0) == "0.0 B"
    assert format_bytes(1024 * 1024) == "1.0 MB"
