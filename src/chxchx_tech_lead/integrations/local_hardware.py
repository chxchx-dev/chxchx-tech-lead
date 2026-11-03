"""Detect what a machine can offer to a local model: GPU memory, RAM and disk.

Detection only reads: `nvidia-smi`, the amdgpu sysfs counters and the platform
architecture. When nothing accelerated is found the machine is treated as
CPU-only, which is slow but never wrong about what fits.
"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

_UNIFIED_GPU_SHARE = 0.66
_AMD_VENDOR_ID = "0x1002"
_MIB = 1024 * 1024

Runner = Callable[..., "subprocess.CompletedProcess[str]"]


@dataclass(frozen=True, slots=True)
class Gpu:
    name: str
    vram_bytes: int
    vendor: str  # "nvidia" | "amd"


@dataclass(frozen=True, slots=True)
class HardwareProfile:
    total_ram_bytes: int
    available_ram_bytes: int
    cpu_cores: int
    disk_free_bytes: int
    models_dir: Path
    gpus: tuple[Gpu, ...] = ()
    unified_memory: bool = False

    @property
    def accelerated_bytes(self) -> int:
        """Memory a model can use at GPU speed."""
        if self.unified_memory:
            return int(self.total_ram_bytes * _UNIFIED_GPU_SHARE)
        return max((gpu.vram_bytes for gpu in self.gpus), default=0)

    @property
    def accelerator(self) -> str:
        if self.unified_memory:
            return "Apple Silicon (memoria unificada)"
        return ", ".join(f"{gpu.name} ({gpu.vram_bytes / 1024**3:.0f} GB)" for gpu in self.gpus) or "sin GPU compatible"


def detect_hardware(
    total_ram_bytes: int,
    available_ram_bytes: int,
    *,
    models_dir: Path | None = None,
    run: Runner = subprocess.run,
) -> HardwareProfile:
    folder = models_dir or default_models_dir()
    return HardwareProfile(
        total_ram_bytes=total_ram_bytes,
        available_ram_bytes=available_ram_bytes,
        cpu_cores=os.cpu_count() or 1,
        disk_free_bytes=_free_bytes(folder),
        models_dir=folder,
        gpus=_nvidia_gpus(run) + _amd_gpus(Path("/sys/class/drm")),
        unified_memory=sys.platform == "darwin" and platform.machine() == "arm64",
    )


def default_models_dir() -> Path:
    override = os.environ.get("OLLAMA_MODELS")
    if override:
        return Path(override).expanduser()
    user_dir = Path.home() / ".ollama" / "models"
    if sys.platform.startswith("linux") and not user_dir.exists():
        return Path("/usr/share/ollama/.ollama/models")  # the systemd service installs here
    return user_dir


def _free_bytes(folder: Path) -> int:
    target = folder
    while not target.exists() and target != target.parent:
        target = target.parent
    try:
        return shutil.disk_usage(target).free
    except OSError:
        return 0


def _nvidia_gpus(run: Runner) -> tuple[Gpu, ...]:
    if shutil.which("nvidia-smi") is None:
        return ()
    try:
        result = run(["nvidia-smi", "--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
                     capture_output=True, text=True, timeout=5, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return ()
    if result.returncode != 0:
        return ()
    gpus = []
    for line in result.stdout.splitlines():
        name, _, memory = line.rpartition(",")
        if name.strip() and memory.strip().isdigit():
            gpus.append(Gpu(name.strip(), int(memory) * _MIB, "nvidia"))
    return tuple(gpus)


def _amd_gpus(drm: Path) -> tuple[Gpu, ...]:
    gpus = []
    try:
        cards = sorted(drm.glob("card[0-9]*/device"))
    except OSError:
        return ()
    for device in cards:
        try:
            if (device / "vendor").read_text().strip() != _AMD_VENDOR_ID:
                continue
            vram = int((device / "mem_info_vram_total").read_text().strip())
        except (OSError, ValueError):
            continue
        gpus.append(Gpu("AMD Radeon", vram, "amd"))
    return tuple(gpus)
