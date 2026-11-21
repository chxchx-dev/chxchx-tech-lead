"""Baseline of the Rust product: CLI startup, Studio idle memory/CPU and binary sizes.

Standard library only. Linux only for the Studio idle sample (reads /proc). Every run uses an
isolated CHXCHX_TECH_HOME so nothing of the user's state is read or written.

    python3 scripts/perf/measure_rust.py --bin target/release --out /tmp/perf --runs 30 --studio-runs 3
"""

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


def create_fixtures(base: Path) -> dict[str, Path]:
    """Deterministic trees: 21 and 5,001 indexable files, plus excluded folders with sentinels."""
    trees = {"small": 20, "large": 5000}
    out = {}
    for name, files in trees.items():
        root = base / name
        if root.exists():
            out[name] = root
            continue
        (root / "src").mkdir(parents=True)
        (root / "README.md").write_text("# fixture\n")
        for index in range(files):
            folder = root / "src" / f"m{index // 100:03d}"
            folder.mkdir(exist_ok=True)
            (folder / f"f{index:05d}.rs").write_text(f"pub fn f{index}() -> usize {{ {index} }}\n")
        for excluded in (".git", "target", "node_modules"):
            (root / excluded).mkdir()
            (root / excluded / "SENTINEL").write_text("must not be indexed\n")
        out[name] = root
    return out


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, round(fraction * (len(ordered) - 1)))]


def cli_startup(binary: Path, project: Path, runs: int, env: dict[str, str]) -> dict[str, float]:
    samples = []
    for _ in range(runs + 1):  # the first run warms the file system cache and is dropped
        start = time.perf_counter()
        subprocess.run([str(binary), "status", str(project)], env=env, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        samples.append((time.perf_counter() - start) * 1000)
    samples = samples[1:]
    return {"p50_ms": statistics.median(samples), "p95_ms": percentile(samples, 0.95)}


def proc_rss_kib(pid: int) -> int:
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmRSS:"):
            return int(line.split()[1])
    return 0


def proc_cpu_ticks(pid: int) -> int:
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return int(fields[11]) + int(fields[12])  # utime + stime


def studio_idle(binary: Path, project: Path, runs: int, warmup: float, samples: int,
                env: dict[str, str]) -> dict[str, float]:
    rss, cpu = [], []
    ticks_per_second = os.sysconf("SC_CLK_TCK")
    for _ in range(runs):
        process = subprocess.Popen([str(binary), str(project)], env=env,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            time.sleep(warmup)
            if process.poll() is not None:
                raise RuntimeError(f"Studio exited early with {process.returncode}")
            before = proc_cpu_ticks(process.pid)
            for _ in range(samples):
                time.sleep(1)
                rss.append(proc_rss_kib(process.pid) / 1024)
            used = (proc_cpu_ticks(process.pid) - before) / ticks_per_second
            cpu.append(100 * used / samples)
        finally:
            process.terminate()
            process.wait(timeout=10)
    return {"rss_median_mib": statistics.median(rss), "rss_max_mib": max(rss),
            "cpu_percent_max": max(cpu)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin", type=Path, default=Path("target/release"))
    parser.add_argument("--out", type=Path, default=Path(tempfile.gettempdir()) / "chxchx-perf")
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--studio-runs", type=int, default=3, help="0 skips Studio (needs a display)")
    parser.add_argument("--warmup", type=float, default=5.0)
    parser.add_argument("--samples", type=int, default=10)
    args = parser.parse_args()

    fixtures = create_fixtures(args.out / "fixtures")
    home = args.out / "home"
    home.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "CHXCHX_TECH_HOME": str(home)}
    cli = args.bin / "chxchx-tech"
    studio = args.bin / "chxchx-studio"

    report: dict[str, object] = {"sizes_bytes": {
        path.name: path.stat().st_size for path in (cli, studio) if path.exists()}}
    report["cli_status"] = {name: cli_startup(cli, root, args.runs, env)
                            for name, root in fixtures.items()}
    if args.studio_runs and sys.platform.startswith("linux"):
        report["studio_idle"] = {name: studio_idle(studio, root, args.studio_runs, args.warmup,
                                                   args.samples, env)
                                 for name, root in fixtures.items()}
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
