from pathlib import Path

from chxchx_tech_lead.workspace.models import ProcessConfig, ResourceConfig
from chxchx_tech_lead.workspace.process_manager import ManagedProcess, ProcessStatus
from chxchx_tech_lead.workspace.resources import (
    ProcessResources,
    ResourceManager,
    ResourceSeverity,
    SystemResources,
    format_bytes,
    summarize_process_resources,
)


def test_resource_manager_classifies_memory_and_swap_thresholds():
    config = ResourceConfig(warn_memory_percent=75, critical_memory_percent=90, warn_swap_percent=40)
    manager = ResourceManager(config, system_reader=lambda: SystemResources(100, 80, 20, 100, 10, 25))

    snapshot = manager.system()

    assert manager.severity(snapshot) is ResourceSeverity.WARNING
    critical = SystemResources(100, 70, 30, 100, 95, 25)
    assert manager.severity(critical) is ResourceSeverity.WARNING
    assert manager.severity(SystemResources(100, 95, 5)) is ResourceSeverity.CRITICAL


def test_agent_start_warnings_cover_memory_and_agent_budget():
    manager = ResourceManager(
        ResourceConfig(warn_memory_percent=70, critical_memory_percent=85, max_agents=2)
    )
    high_memory = SystemResources(100, 88, 12, 100, 50)

    warnings = manager.agent_start_warnings(1, 2, high_memory)

    assert len(warnings) == 2
    assert "RAM al 88%" in warnings[0]
    assert "3 agentes activos" in warnings[1]
    assert manager.agent_start_warnings(2, 0, high_memory) == ()


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


def test_process_resource_summary_sums_running_managed_processes_only(tmp_path: Path):
    summary = summarize_process_resources(
        [
            ProcessResources("api", "API", 10, 20_000, 12.5, ProcessStatus.RUNNING),
            ProcessResources("web", "Web", 11, 30_000, 5.0, ProcessStatus.RUNNING),
            ProcessResources("worker", "Worker", None, 0, None, ProcessStatus.STOPPED),
        ]
    )

    assert summary.running_count == 2
    assert summary.rss_bytes == 50_000
    assert summary.cpu_percent == 17.5
    assert summary.measured_cpu_count == 2
    assert summary.memory_percent(1_000_000) == 5
