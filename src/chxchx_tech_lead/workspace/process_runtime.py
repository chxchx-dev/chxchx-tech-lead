"""Operaciones de bajo nivel específicas del sistema operativo para procesos."""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


def command_available(command: list[str] | str, cwd: Path, shell: bool) -> bool:
    if shell:
        return isinstance(command, str) and bool(command.strip())
    if not isinstance(command, list) or not command:
        return False
    executable = command[0]
    if "/" in executable or "\\" in executable:
        return (cwd / executable).exists() if not Path(executable).is_absolute() else Path(executable).exists()
    return shutil.which(executable) is not None


def pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except (OSError, ProcessLookupError):
        return False
    return True


def terminate_posix_process_group(
    process_group_id: int,
    handle: subprocess.Popen[Any] | None,
    *,
    timeout_seconds: float,
) -> None:
    """Stop only the isolated process group created for this managed process."""
    if not _signal_process_group(process_group_id, signal.SIGTERM):
        return
    if handle is not None:
        try:
            handle.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            _signal_process_group(process_group_id, signal.SIGKILL)
            handle.wait(timeout=timeout_seconds)
            return

    deadline = time.monotonic() + timeout_seconds
    while _process_group_alive(process_group_id) and time.monotonic() < deadline:
        time.sleep(0.05)
    if _process_group_alive(process_group_id):
        _signal_process_group(process_group_id, signal.SIGKILL)
        if handle is not None and handle.poll() is None:
            handle.wait(timeout=timeout_seconds)


def terminate_windows_process_tree(
    pid: int,
    handle: subprocess.Popen[Any] | None,
    *,
    timeout_seconds: float,
) -> None:
    """Stop a managed Windows process and its descendants."""
    command = ["taskkill", "/PID", str(pid), "/T"]
    if handle is None:
        result = subprocess.run(
            [*command, "/F"], check=False, capture_output=True, text=True
        )
        if result.returncode != 0:
            detail = result.stderr.strip() or result.stdout.strip()
            raise OSError(detail or f"taskkill no pudo detener el árbol del PID {pid}")
        return

    result = subprocess.run(command, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        # Windows can reject graceful tree termination when a descendant
        # requires forceful termination. Escalate while the owned root is live.
        if handle.poll() is None:
            result = subprocess.run(
                [*command, "/F"], check=False, capture_output=True, text=True
            )
            if result.returncode == 0:
                handle.wait(timeout=timeout_seconds)
                return
    else:
        try:
            handle.wait(timeout=timeout_seconds)
            return
        except subprocess.TimeoutExpired:
            result = subprocess.run(
                [*command, "/F"], check=False, capture_output=True, text=True
            )
            if result.returncode == 0:
                handle.wait(timeout=timeout_seconds)
                return

    # Retain direct-child cleanup, but report that the whole tree was not
    # confirmed stopped if both taskkill attempts failed.
    if handle.poll() is None:
        handle.terminate()
    handle.wait(timeout=timeout_seconds)
    detail = result.stderr.strip() or result.stdout.strip()
    raise OSError(detail or f"taskkill no pudo confirmar el cierre del árbol del PID {pid}")


def pid_matches_command(pid: int, command: list[str] | str, shell: bool) -> bool:
    if not pid_alive(pid):
        return False
    if shell:
        shell_executable = os.environ.get("COMSPEC", "cmd.exe") if os.name == "nt" else "sh"
        expected_name = Path(shell_executable).name
    else:
        expected = command if isinstance(command, str) else command[0]
        expected_name = Path(expected.split()[0]).name
    if sys.platform.startswith("linux"):
        try:
            raw = Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace").strip()
            return bool(raw) and Path(raw.split()[0]).name == expected_name
        except OSError:
            return False
    if os.name != "nt":
        result = subprocess.run(["ps", "-p", str(pid), "-o", "command="], capture_output=True, text=True, check=False)
        raw = result.stdout.strip()
        return result.returncode == 0 and bool(raw) and Path(raw.split()[0]).name == expected_name
    result = subprocess.run(
        ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0 and expected_name.lower() in result.stdout.lower()


def _process_group_alive(process_group_id: int) -> bool:
    try:
        os.killpg(process_group_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _signal_process_group(process_group_id: int, process_signal: int) -> bool:
    try:
        os.killpg(process_group_id, process_signal)
    except ProcessLookupError:
        return False
    return True
