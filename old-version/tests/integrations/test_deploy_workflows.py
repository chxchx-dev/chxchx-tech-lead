from pathlib import Path

import pytest

from chxchx_tech_lead.integrations.deploy_workflows import plan_deploy_workflow


def test_manual_vps_workflow_is_opt_in_and_has_secure_ssh_defaults(tmp_path: Path) -> None:
    (tmp_path / "compose.yaml").write_text("services: {}\n", encoding="utf-8")

    plan = plan_deploy_workflow(tmp_path)

    assert plan.compose_file == "compose.yaml"
    assert plan.auto_deploy_branch is None
    assert "on:\n  workflow_dispatch:" in plan.content
    assert "default: false" in plan.content
    assert "permissions:\n  contents: read" in plan.content
    assert "    environment: production" in plan.content
    assert "pull_request" not in plan.content
    assert "StrictHostKeyChecking=yes" in plan.content
    assert "VPS_SSH_PRIVATE_KEY" in plan.content
    assert "VPS_SSH_KNOWN_HOSTS" in plan.content
    assert "checkout --detach \"$previous_sha\"" in plan.content
    assert "cancel-in-progress: false" in plan.content


def test_push_deployment_is_only_generated_for_the_explicit_branch(tmp_path: Path) -> None:
    (tmp_path / "docker-compose.yml").write_text("services: {}\n", encoding="utf-8")

    plan = plan_deploy_workflow(tmp_path, "release/production")

    assert "on:\n  push:\n    branches:\n      - 'release/production'" in plan.content
    assert "if: github.event_name != 'workflow_dispatch' || inputs.confirm_deploy" in plan.content
    assert plan.required_variables == ("VPS_SSH_HOST", "VPS_SSH_USER", "VPS_DEPLOY_PATH")
    assert plan.required_secrets == ("VPS_SSH_PRIVATE_KEY", "VPS_SSH_KNOWN_HOSTS")


@pytest.mark.parametrize("branch", ["-main", "main*", "release//prod", "release/../prod", "main\nrun: bad"])
def test_invalid_auto_deploy_branch_is_rejected(tmp_path: Path, branch: str) -> None:
    (tmp_path / "compose.yml").write_text("services: {}\n", encoding="utf-8")

    with pytest.raises(ValueError, match="rama de despliegue"):
        plan_deploy_workflow(tmp_path, branch)


def test_requires_exactly_one_compose_manifest(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="Docker Compose"):
        plan_deploy_workflow(tmp_path)

    (tmp_path / "compose.yaml").write_text("services: {}\n", encoding="utf-8")
    (tmp_path / "compose.prod.yaml").write_text("services: {}\n", encoding="utf-8")
    (tmp_path / "docker-compose.yml").write_text("services: {}\n", encoding="utf-8")
    with pytest.raises(ValueError, match="varios manifiestos"):
        plan_deploy_workflow(tmp_path)
