from pathlib import Path

import pytest

from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.integrations import github_cli
from chxchx_tech_lead.integrations.github_cli import GitHubCliError


def test_dispatch_workflow_dry_run_is_argument_safe_and_explicit(tmp_path: Path) -> None:
    result = github_cli.dispatch_workflow(
        tmp_path,
        "chxchx-deploy.yml",
        "release/production",
        fields={"confirm_deploy": "true"},
        dry_run=True,
    )

    assert result.command == [
        "gh", "workflow", "run", "chxchx-deploy.yml", "--ref", "release/production",
        "-f", "confirm_deploy=true",
    ]
    assert "no se contactó GitHub" in result.stdout


@pytest.mark.parametrize(
    ("workflow", "ref", "fields"),
    [
        ("../deploy.yml", None, None),
        ("chxchx-deploy.yml", "main; rm -rf /", None),
        ("chxchx-deploy.yml", None, {"confirm_deploy": "true\n--ref=main"}),
        ("chxchx-deploy.yml", None, {"bad-field": "true"}),
    ],
)
def test_dispatch_workflow_rejects_untrusted_arguments(tmp_path: Path, workflow, ref, fields) -> None:
    with pytest.raises(GitHubCliError):
        github_cli.dispatch_workflow(tmp_path, workflow, ref, fields=fields, dry_run=True)


def test_recent_runs_can_filter_to_the_deploy_workflow(tmp_path: Path, monkeypatch) -> None:
    captured: dict[str, object] = {}

    def fake_gh(root, args, *, timeout):
        captured["args"] = args
        return CommandResult(["gh", *args], 0, "[]", "")

    monkeypatch.setattr(github_cli, "_gh", fake_gh)
    assert github_cli.recent_runs(tmp_path, workflow="chxchx-deploy.yml") == []
    assert "chxchx-deploy.yml" in captured["args"]
