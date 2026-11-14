from pathlib import Path

import pytest

from chxchx_tech_lead.integrations import ci_workflow_files, deploy_service
from chxchx_tech_lead.integrations.github_cli import GitHubCliError


def _compose_project(path: Path) -> Path:
    (path / "compose.yaml").write_text("services: {}\n", encoding="utf-8")
    return path


def test_setup_is_dry_run_by_default_and_manages_its_own_workflow(tmp_path: Path, monkeypatch) -> None:
    project = _compose_project(tmp_path)
    backup_home = tmp_path / "chxchx-home"
    monkeypatch.setattr(ci_workflow_files, "ensure_home", lambda: backup_home)

    preview, plan = deploy_service.setup_deployment(project, dry_run=True)
    assert preview.changed is True
    assert not preview.path.exists()
    assert plan.path == Path(".github/workflows/chxchx-deploy.yml")

    written, _ = deploy_service.setup_deployment(project)
    assert written.changed is True
    generated = written.path.read_text(encoding="utf-8")
    assert generated.endswith(plan.content.rstrip() + "\n")
    assert ci_workflow_files.current_workflow_state(project, plan.path).integrity_ok
    assert deploy_service.setup_deployment(project)[0].changed is False

    removed = deploy_service.remove_deployment(project)
    assert removed.changed is True
    assert removed.backup and removed.backup.is_file()
    assert not removed.path.exists()


def test_deploy_workflow_collision_or_external_edit_is_preserved(tmp_path: Path) -> None:
    project = _compose_project(tmp_path)
    workflow = project / ".github/workflows/chxchx-deploy.yml"
    workflow.parent.mkdir(parents=True)
    workflow.write_text("# Maintainer workflow\n", encoding="utf-8")

    with pytest.raises(ValueError, match="no está administrado"):
        deploy_service.setup_deployment(project)
    assert workflow.read_text(encoding="utf-8") == "# Maintainer workflow\n"


def test_manual_dispatch_requires_confirmation_and_passes_confirmation_field(tmp_path: Path, monkeypatch) -> None:
    project = _compose_project(tmp_path)
    deploy_service.setup_deployment(project)
    captured: dict[str, object] = {}

    def dispatch(root, workflow, ref, *, fields, dry_run):
        captured.update(root=root, workflow=workflow, ref=ref, fields=fields, dry_run=dry_run)
        return deploy_service.CommandResult(["gh"], 0, "queued", "")

    monkeypatch.setattr(deploy_service, "dispatch_workflow", dispatch)
    with pytest.raises(GitHubCliError, match="confirmación explícita"):
        deploy_service.run_deployment(project)

    result = deploy_service.run_deployment(project, "main", confirmed=True)
    assert result.stdout == "queued"
    assert captured["workflow"] == "chxchx-deploy.yml"
    assert captured["ref"] == "main"
    assert captured["fields"] == {"confirm_deploy": "true"}


def test_deploy_dispatch_rechecks_compose_manifest(tmp_path: Path, monkeypatch) -> None:
    project = _compose_project(tmp_path)
    deploy_service.setup_deployment(project)
    (project / "compose.yaml").unlink()
    dispatched = False

    def dispatch(*args, **kwargs):
        nonlocal dispatched
        dispatched = True
        raise AssertionError("No se debe despachar sin Compose")

    monkeypatch.setattr(deploy_service, "dispatch_workflow", dispatch)
    with pytest.raises(GitHubCliError, match="Docker Compose"):
        deploy_service.run_deployment(project, confirmed=True)
    assert dispatched is False


def test_status_is_read_only_and_lists_required_vps_configuration(tmp_path: Path, monkeypatch) -> None:
    project = _compose_project(tmp_path)
    monkeypatch.setattr(deploy_service, "authenticated", lambda root: False)

    payload = deploy_service.deployment_status_payload(project)

    assert payload["schema"] == "chxchx.deploy-status"
    assert payload["configured"] is False
    assert "VPS_SSH_PRIVATE_KEY" in payload["required_secrets"]
    assert "VPS_DEPLOY_PATH" in payload["required_variables"]
    assert not (project / ".github").exists()


def test_status_reports_automatic_branch_from_integral_managed_workflow(tmp_path: Path, monkeypatch) -> None:
    project = _compose_project(tmp_path)
    monkeypatch.setattr(deploy_service, "authenticated", lambda root: False)
    deploy_service.setup_deployment(project, "release/production")

    payload = deploy_service.deployment_status_payload(project)

    assert payload["auto_deploy_branch"] == "release/production"
