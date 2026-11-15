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
    timeout: float | None = None,
) -> CommandResult:
    cmd = [str(part) for part in command]
    if dry_run:
        return CommandResult(cmd, 0, "DRY RUN", "")
    if interactive:
        # Keep stdout attached to the user's TTY for full-screen programs,
        # while capturing stderr so adapters can report failed launches.
        try:
            proc = subprocess.run(
                cmd,
                cwd=cwd,
                text=True,
                stdout=None,
                stderr=subprocess.PIPE,
                check=False,
                timeout=timeout,
            )
        except subprocess.TimeoutExpired as exc:
            stderr = _decode_output(exc.stderr)
            limit = f"{timeout:g}" if timeout is not None else "el límite configurado"
            detail = f"El comando superó el límite de {limit} segundos."
            if stderr:
                detail = f"{detail} {stderr}"
            return CommandResult(cmd, 124, "", detail)
        return CommandResult(cmd, proc.returncode, "", (proc.stderr or "").strip())
    try:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            text=True,
            capture_output=True,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        stdout = _decode_output(exc.stdout)
        stderr = _decode_output(exc.stderr)
        detail = f"El comando superó el límite de {timeout:g} segundos."
        if stderr:
            detail = f"{detail} {stderr}"
        return CommandResult(cmd, 124, stdout, detail)
    return CommandResult(
        cmd,
        proc.returncode,
        (proc.stdout or "").strip(),
        (proc.stderr or "").strip(),
    )


def _decode_output(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode(errors="replace").strip()
    return (value or "").strip()
