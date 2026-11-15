from pathlib import Path

from chxchx_tech_lead.adapters.git.cli import GitAdapter
from chxchx_tech_lead.core.runner import CommandResult


def test_git_adapter_parses_read_only_status(tmp_path: Path):
    responses = {
        "branch": CommandResult([], 0, "feature/workspace", ""),
        "status": CommandResult([], 0, " M src/app.py\n?? notes.txt\n", ""),
        "upstream": CommandResult([], 0, "origin/feature/workspace", ""),
        "rev-list": CommandResult([], 0, "2\t3", ""),
    }
    calls = []

    def fake_runner(command, **kwargs):
        calls.append(command)
        if command[3] == "branch":
            return responses["branch"]
        if command[3] == "status":
            return responses["status"]
        if command[3] == "rev-parse":
            return responses["upstream"]
        return responses["rev-list"]

    status = GitAdapter(tmp_path, runner=fake_runner, lookup=lambda _: "/usr/bin/git").status()

    assert status.branch == "feature/workspace"
    assert status.dirty is True
    assert status.changed_count == 2
    assert status.upstream == "origin/feature/workspace"
    assert status.ahead == 3
    assert status.behind == 2
    assert all(command[0:2] == ["git", "-C"] for command in calls)
