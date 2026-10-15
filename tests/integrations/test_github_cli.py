from __future__ import annotations

import json
from pathlib import Path

import pytest

from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.integrations import github_cli


def test_ci_dispatch_dry_run_does_not_contact_github(tmp_path: Path, monkeypatch) -> None:
    calls = []
    monkeypatch.setattr(github_cli, "executable", lambda _name: "/usr/bin/gh")
    monkeypatch.setattr(github_cli, "run", lambda *args, **kwargs: calls.append(args) or None)

    preview = github_cli.dispatch_ci(tmp_path, "feature/ci", dry_run=True)

    assert "chxchx-ci.yml" in preview.command
    assert preview.returncode == 0
    assert calls == []


def test_ci_dispatch_uses_gh_and_validates_ref(tmp_path: Path, monkeypatch) -> None:
    calls = []

    def fake_run(command, **kwargs):
        calls.append(command)
        if command[-2:] == ["auth", "status"]:
            return CommandResult(command, 0, "authenticated as example", "")
        return CommandResult(command, 0, "https://github.com/example/project/actions/runs/42", "")

    monkeypatch.setattr(github_cli, "executable", lambda _name: "/usr/bin/gh")
    monkeypatch.setattr(github_cli, "run", fake_run)
    result = github_cli.dispatch_ci(tmp_path, "feature/ci")

    assert result.returncode == 0
    assert calls[-1][-3:] == ["chxchx-ci.yml", "--ref", "feature/ci"]
    with pytest.raises(github_cli.GitHubCliError, match="referencia de Git"):
        github_cli.dispatch_ci(tmp_path, "bad ref", dry_run=True)


def test_recent_runs_requests_only_known_fields(tmp_path: Path, monkeypatch) -> None:
    calls = []

    def fake_run(command, **kwargs):
        calls.append(command)
        return CommandResult(command, 0, json.dumps([{"databaseId": 42, "status": "completed"}]), "")

    monkeypatch.setattr(github_cli, "executable", lambda _name: "/usr/bin/gh")
    monkeypatch.setattr(github_cli, "run", fake_run)

    runs = github_cli.recent_runs(tmp_path)

    assert runs == [{"databaseId": 42, "status": "completed"}]
    assert calls[0][1:4] == ["run", "list", "--workflow"]
    assert calls[0][4] == "chxchx-ci.yml"
