"""Starting and stopping the provider process, and everything it starts."""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import threading
import time
from collections import deque
from typing import IO, Any

_STOP_TIMEOUT_SECONDS = 3.0


def start(argv: list[str], cwd: Any, env: dict[str, str]) -> subprocess.Popen[str]:
    """The provider process, alone in its own group so stopping it stops what it launched."""
    # A bare name is looked up in the PATH the process will get; on Windows that also finds `codex.cmd`.
    resolved = shutil.which(argv[0], path=env.get("PATH") or env.get("Path")) or argv[0]
    return subprocess.Popen(
        [resolved, *argv[1:]],
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        start_new_session=os.name != "nt",
        creationflags=0x0800_0000 if os.name == "nt" else 0,  # CREATE_NO_WINDOW
        env=env,
    )


def drain(stream: IO[str] | None, tail: deque[str]) -> None:
    """Keeps the last lines of a stream, for the message when the provider fails."""
    if stream is None:
        return
    for line in stream:
        if line.strip():
            tail.append(line.rstrip())


def stop(process: subprocess.Popen[str]) -> None:
    """Politely first, by force after a few seconds. Never raises."""
    if process.poll() is not None:
        return
    try:
        if os.name == "nt":
            _stop_windows(process)
        else:
            _stop_posix(process)
    except (OSError, subprocess.TimeoutExpired):
        process.kill()


def _stop_posix(process: subprocess.Popen[str]) -> None:
    group = process.pid
    if not _signal_group(group, signal.SIGTERM):
        return
    try:
        process.wait(timeout=_STOP_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        _signal_group(group, signal.SIGKILL)
        process.wait(timeout=_STOP_TIMEOUT_SECONDS)
        return
    deadline = time.monotonic() + _STOP_TIMEOUT_SECONDS
    while _group_alive(group) and time.monotonic() < deadline:
        time.sleep(0.05)
    if _group_alive(group):
        _signal_group(group, signal.SIGKILL)


def _stop_windows(process: subprocess.Popen[str]) -> None:
    command = ["taskkill", "/PID", str(process.pid), "/T"]
    if subprocess.run(command, check=False, capture_output=True, text=True).returncode != 0 and process.poll() is None:
        subprocess.run([*command, "/F"], check=False, capture_output=True, text=True)
    try:
        process.wait(timeout=_STOP_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        subprocess.run([*command, "/F"], check=False, capture_output=True, text=True)


def _group_alive(group: int) -> bool:
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _signal_group(group: int, sent: int) -> bool:
    try:
        os.killpg(group, sent)
    except ProcessLookupError:
        return False
    return True


def watch_control(source: IO[str], process: subprocess.Popen[str], cancelled: threading.Event, live: Any) -> None:
    """Relays answers to the live driver; `cancel` or the end of the input stops the provider."""
    for line in source:
        text = line.strip()
        if text == "cancel":
            break
        if text.startswith("{"):
            try:
                message = json.loads(text)
            except json.JSONDecodeError:
                continue
            if isinstance(message, dict) and message.get("type") == "answer":
                live.answer(message)
    # Whoever drives this service went away: never leave the provider running.
    if process.poll() is not None:
        return
    cancelled.set()
    stop(process)
