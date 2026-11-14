import subprocess

from chxchx_tech_lead.core.runner import run


def test_run_returns_actionable_result_when_command_times_out(monkeypatch):
    def timeout(*_args, **_kwargs):
        raise subprocess.TimeoutExpired(
            cmd=["slow-tool", "--status"],
            timeout=2,
            output=b"partial output",
            stderr=b"still waiting",
        )

    monkeypatch.setattr(subprocess, "run", timeout)

    result = run(["slow-tool", "--status"], timeout=2)

    assert result.returncode == 124
    assert result.stdout == "partial output"
    assert "superó el límite de 2 segundos" in result.stderr
    assert "still waiting" in result.stderr


def test_run_dry_run_does_not_start_a_subprocess(monkeypatch):
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("subprocess called")),
    )

    result = run(["slow-tool", "--status"], dry_run=True, timeout=2)

    assert result.returncode == 0
    assert result.stdout == "DRY RUN"


def test_interactive_run_keeps_stdout_attached_to_the_terminal(monkeypatch):
    received = {}

    def fake_run(_command, **kwargs):
        received.update(kwargs)
        return subprocess.CompletedProcess([], 0, None, "")

    monkeypatch.setattr(subprocess, "run", fake_run)

    result = run(["zellij", "attach", "project"], interactive=True)

    assert received["stdout"] is None
    assert received["stderr"] == subprocess.PIPE
    assert result.returncode == 0
