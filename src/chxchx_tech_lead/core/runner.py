from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
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


def run(
    command: Sequence[str],
    dry_run: bool = False,
    cwd: Path | None = None,
    interactive: bool = False,
) -> CommandResult:
    cmd = [str(part) for part in command]
    if dry_run:
        return CommandResult(cmd, 0, "DRY RUN", "")
    if interactive:
        # Keep stdout attached to the user's TTY for full-screen programs,
        # while capturing stderr so adapters can report failed launches.
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            text=True,
            stdout=None,
            stderr=subprocess.PIPE,
            check=False,
        )
        return CommandResult(cmd, proc.returncode, "", (proc.stderr or "").strip())
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=not interactive, check=False)
    return CommandResult(
        cmd,
        proc.returncode,
        (proc.stdout or "").strip(),
        (proc.stderr or "").strip(),
    )
