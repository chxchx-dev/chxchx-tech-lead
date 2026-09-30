from __future__ import annotations

import subprocess

from chxchx_tech_lead.workspace import process_runtime


class FakeHandle:
    def __init__(self, *, timeouts: int = 0):
        self.returncode = None
        self.timeouts = timeouts
        self.wait_calls = 0
        self.terminated = False

    def poll(self):
        return self.returncode

    def wait(self, timeout=None):
        self.wait_calls += 1
        if self.wait_calls <= self.timeouts:
            raise subprocess.TimeoutExpired("managed", timeout)
        self.returncode = 0
        return self.returncode

    def terminate(self):
        self.terminated = True
        self.returncode = 0


def test_windows_tree_stop_uses_taskkill_before_waiting_for_owned_child(monkeypatch):
    calls = []
    monkeypatch.setattr(
        process_runtime.subprocess,
        "run",
        lambda command, **kwargs: calls.append((command, kwargs))
        or subprocess.CompletedProcess(command, 0, "", ""),
    )
    handle = FakeHandle()

    process_runtime.terminate_windows_process_tree(4242, handle, timeout_seconds=5)

    assert calls[0][0] == ["taskkill", "/PID", "4242", "/T"]
    assert "/F" not in calls[0][0]
    assert handle.wait_calls == 1
    assert not handle.terminated


def test_windows_tree_stop_forces_only_after_graceful_timeout(monkeypatch):
    calls = []
    monkeypatch.setattr(
        process_runtime.subprocess,
        "run",
        lambda command, **kwargs: calls.append(command)
        or subprocess.CompletedProcess(command, 0, "", ""),
    )
    handle = FakeHandle(timeouts=1)

    process_runtime.terminate_windows_process_tree(4343, handle, timeout_seconds=0)

    assert calls == [
        ["taskkill", "/PID", "4343", "/T"],
        ["taskkill", "/PID", "4343", "/T", "/F"],
    ]
    assert handle.wait_calls == 2
    assert not handle.terminated


def test_recovered_windows_process_uses_forced_tree_stop(monkeypatch):
    calls = []
    monkeypatch.setattr(
        process_runtime.subprocess,
        "run",
        lambda command, **kwargs: calls.append(command)
        or subprocess.CompletedProcess(command, 0, "", ""),
    )

    process_runtime.terminate_windows_process_tree(4444, None, timeout_seconds=5)

    assert calls == [["taskkill", "/PID", "4444", "/T", "/F"]]


def test_windows_tree_stop_reports_unconfirmed_descendants(monkeypatch):
    monkeypatch.setattr(
        process_runtime.subprocess,
        "run",
        lambda command, **kwargs: subprocess.CompletedProcess(
            command, 1, "", "acceso denegado"
        ),
    )
    handle = FakeHandle()

    try:
        process_runtime.terminate_windows_process_tree(4545, handle, timeout_seconds=5)
    except OSError as exc:
        assert "acceso denegado" in str(exc)
    else:
        raise AssertionError("no debe reportar éxito si taskkill no confirma el cierre del árbol")

    assert handle.terminated
    assert handle.returncode == 0
