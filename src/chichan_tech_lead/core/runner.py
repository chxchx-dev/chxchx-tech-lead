from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass
from typing import Sequence


@dataclass(slots=True)
class CommandResult:
    command: list[str]
    returncode: int
    stdout: str
    stderr: str
    skipped: bool = False


def executable(name: str) -> str | None:
    return shutil.which(name)


def run(command: Sequence[str], dry_run: bool = False) -> CommandResult:
    cmd = [str(part) for part in command]
    if dry_run:
        return CommandResult(cmd, 0, "DRY RUN", "")
    proc = subprocess.run(cmd, text=True, capture_output=True, check=False)
    return CommandResult(cmd, proc.returncode, proc.stdout.strip(), proc.stderr.strip())
