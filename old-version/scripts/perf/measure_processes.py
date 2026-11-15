from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Callable

import psutil


def percentile(samples: list[float], fraction: float) -> float:
    ordered = sorted(samples)
    index = max(0, min(len(ordered) - 1, int((len(ordered) * fraction + 0.999999)) - 1))
    return ordered[index]


def summary(samples: list[float]) -> dict[str, float]:
    return {
        "p50_ms": percentile(samples, 0.50),
        "p95_ms": percentile(samples, 0.95),
    }


def isolated_environment(root: Path, label: str, *, studio: bool) -> dict[str, str]:
    env = os.environ.copy()
    env["CHXCHX_TECH_HOME"] = str(root / "state" / label)
    if studio:
        env.pop("LD_LIBRARY_PATH", None)
        env["QT_QPA_PLATFORM"] = "offscreen"
        env["XDG_CONFIG_HOME"] = str(root / "xdg-config" / label)
        env["XDG_CACHE_HOME"] = str(root / "xdg-cache" / label)
        env["XDG_DATA_HOME"] = str(root / "xdg-data" / label)
    return env


def timed_cli(cli: Path, project: Path, scratch: Path, runs: int) -> dict[str, float]:
    samples: list[float] = []
    env = isolated_environment(scratch, project.name, studio=False)
    for _ in range(runs):
        started = time.perf_counter_ns()
        result = subprocess.run(
            [str(cli), "status", str(project)],
            cwd=project,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
        samples.append((time.perf_counter_ns() - started) / 1_000_000.0)
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors="replace"))
    return summary(samples)


def studio_command(studio: Path, project: Path, env: dict[str, str]) -> list[str]:
    return [str(studio), str(project)]


def timed_studio_startup(
    studio: Path,
    project: Path,
    scratch: Path,
    runs: int,
    delay_ms: int,
) -> dict[str, float]:
    samples: list[float] = []
    env = isolated_environment(scratch, project.name, studio=True)
    for index in range(runs):
        capture = scratch / f"{project.name}-{index}.png"
        env["CHXCHX_STUDIO_SCREENSHOT"] = str(capture)
        env["CHXCHX_STUDIO_SCREENSHOT_DELAY_MS"] = str(delay_ms)
        started = time.perf_counter_ns()
        result = subprocess.run(
            studio_command(studio, project, env),
            cwd=project,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
        elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000.0
        if result.returncode or not capture.is_file():
            raise RuntimeError(
                f"Studio launch {index + 1}/{runs} failed: exit={result.returncode}, "
                f"screenshot_saved={capture.is_file()}, "
                f"stderr={result.stderr.decode(errors='replace').strip() or '<empty>'}"
            )
        samples.append(max(0.0, elapsed_ms - delay_ms))
    return summary(samples)


def studio_idle(studio: Path, project: Path, scratch: Path, samples_count: int) -> dict[str, float]:
    env = isolated_environment(scratch, project.name, studio=True)
    env["CHXCHX_STUDIO_SCREENSHOT"] = str(scratch / f"{project.name}-idle.png")
    env["CHXCHX_STUDIO_SCREENSHOT_DELAY_MS"] = "120000"
    process = subprocess.Popen(
        studio_command(studio, project, env),
        cwd=project,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    observed = psutil.Process(process.pid)
    try:
        time.sleep(5.0)
        rss_samples: list[float] = []
        cpu_start = observed.cpu_times()
        elapsed_start = time.perf_counter()
        for _ in range(samples_count):
            time.sleep(1.0)
            rss_samples.append(observed.memory_info().rss / (1024.0 * 1024.0))
        elapsed = time.perf_counter() - elapsed_start
        cpu_end = observed.cpu_times()
        cpu_seconds = (cpu_end.user + cpu_end.system) - (cpu_start.user + cpu_start.system)
        return {
            "rss_median_mib": statistics.median(rss_samples),
            "rss_max_mib": max(rss_samples),
            "process_cpu_percent_over_sample": 100.0 * cpu_seconds / elapsed,
        }
    finally:
        observed.terminate()
        try:
            observed.wait(timeout=5)
        except psutil.TimeoutExpired:
            observed.kill()
            observed.wait(timeout=5)


def measure(
    function: Callable[[Path], dict[str, float]],
    projects: list[Path],
) -> dict[str, dict[str, float]]:
    return {project.name: function(project) for project in projects}


def main() -> None:
    parser = argparse.ArgumentParser(description="Measure legacy CLI and Studio startup/idle metrics on Linux.")
    parser.add_argument("--small", type=Path, required=True)
    parser.add_argument("--large", type=Path, required=True)
    parser.add_argument("--studio", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--idle-samples", type=int, default=10)
    parser.add_argument("--screenshot-delay-ms", type=int, default=500)
    args = parser.parse_args()
    projects = [args.small.resolve(), args.large.resolve()]
    studio = args.studio.resolve()
    cli = Path(sys.executable).with_name("chxchx-tech")
    if not cli.is_file():
        parser.error(f"could not find project CLI next to Python interpreter: {cli}")
    if not studio.is_file():
        parser.error(f"Studio executable not found: {studio}")
    if args.runs < 2 or args.idle_samples < 2 or args.screenshot_delay_ms < 100:
        parser.error("runs and idle-samples must be >= 2; screenshot delay must be >= 100 ms")

    with tempfile.TemporaryDirectory(prefix="chxchx-process-bench-") as temporary:
        scratch = Path(temporary)
        result = {
            "python_cli_status": measure(
                lambda project: timed_cli(cli, project, scratch, args.runs), projects
            ),
            "studio_window_startup_proxy": measure(
                lambda project: timed_studio_startup(
                    studio, project, scratch, args.runs, args.screenshot_delay_ms
                ),
                projects,
            ),
            "studio_idle_offscreen": measure(
                lambda project: studio_idle(studio, project, scratch, args.idle_samples), projects
            ),
            "method": {
                "cli": f"{cli} status PATH; {args.runs} warm process launches per fixture",
                "studio_startup": (
                    "window screenshot auto-exit elapsed minus fixed delay; offscreen; "
                    "proxy, not instrumented first-frame latency"
                ),
                "studio_idle": (
                    f"offscreen; 5 s warmup, {args.idle_samples} one-second RSS samples; "
                    "CPU is process CPU over same sample"
                ),
                "filesystem_cache": "warm after first run; cold-cache data not measured",
            },
        }
        print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
