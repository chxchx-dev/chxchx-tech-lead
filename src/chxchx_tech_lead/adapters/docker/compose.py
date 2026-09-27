from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from ...core.runner import CommandResult, executable, run


@dataclass(frozen=True, slots=True)
class DockerStats:
    name: str
    memory: str
    cpu_percent: str


class DockerComposeAdapter:
    """Adapter mínimo para Compose por proyecto; `down` siempre es explícito."""

    def __init__(
        self,
        project_root: Path,
        compose_file: str = "compose.yaml",
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
    ):
        self.project_root = project_root.expanduser().resolve()
        self.compose_file = compose_file
        self._runner = runner
        self._lookup = lookup

    @property
    def compose_path(self) -> Path:
        return (self.project_root / self.compose_file).resolve()

    def available(self) -> bool:
        return self._lookup("docker") is not None

    def compose_available(self) -> bool:
        if not self.available():
            return False
        result = self._runner(["docker", "compose", "version"], dry_run=False)
        return result.returncode == 0

    def up(self, dry_run: bool = False) -> CommandResult:
        return self._compose_action("up", ["-d"], dry_run)

    def stop(self, dry_run: bool = False) -> CommandResult:
        return self._compose_action("stop", [], dry_run)

    def down(self, dry_run: bool = False) -> CommandResult:
        return self._compose_action("down", [], dry_run)

    def ps(self, dry_run: bool = False) -> CommandResult:
        return self._compose_action("ps", [], dry_run)

    def stats(self, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [
                "docker",
                "stats",
                "--no-stream",
                "--format",
                "{{.Name}}\t{{.MemUsage}}\t{{.CPUPerc}}",
            ],
            dry_run=dry_run,
        )

    @staticmethod
    def parse_stats(output: str) -> list[DockerStats]:
        stats: list[DockerStats] = []
        for line in output.splitlines():
            parts = line.split("\t")
            if len(parts) != 3:
                continue
            name, memory, cpu = (part.strip() for part in parts)
            if name:
                stats.append(DockerStats(name, memory, cpu))
        return stats

    def _compose_action(self, action: str, extra: list[str], dry_run: bool) -> CommandResult:
        command = ["docker", "compose", "-f", str(self.compose_path), action, *extra]
        if not dry_run and not self.compose_path.is_file():
            return CommandResult(command, 2, "", f"No existe el archivo Compose: {self.compose_path}")
        if not dry_run and not self.compose_available():
            return CommandResult(command, 127, "", "Docker Compose no está disponible")
        return self._runner(command, dry_run=dry_run, cwd=self.project_root)
