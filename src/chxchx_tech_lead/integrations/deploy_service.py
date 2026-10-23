"""Use cases for optional Docker Compose deployments to a user's VPS."""

from __future__ import annotations

import re
from pathlib import Path

from ..core.runner import CommandResult
from .ci_workflow_files import WorkflowWriteResult, current_workflow_state, remove_workflow, write_workflow
from .deploy_workflows import DeployWorkflowPlan, WORKFLOW_RELATIVE_PATH, plan_deploy_workflow
from .github_cli import GitHubCliError, authenticated, dispatch_workflow, recent_runs

DEPLOY_STATUS_SCHEMA = "chxchx.deploy-status"


def setup_deployment(
    root: Path, auto_deploy_branch: str | None = None, *, dry_run: bool = False
) -> tuple[WorkflowWriteResult, DeployWorkflowPlan]:
    plan = plan_deploy_workflow(root, auto_deploy_branch)
    return write_workflow(root, plan, dry_run=dry_run), plan


def remove_deployment(root: Path, *, dry_run: bool = False) -> WorkflowWriteResult:
    return remove_workflow(root, WORKFLOW_RELATIVE_PATH, dry_run=dry_run)


def run_deployment(
    root: Path,
    ref: str | None = None,
    *,
    confirmed: bool = False,
    dry_run: bool = False,
) -> CommandResult:
    state = current_workflow_state(root, WORKFLOW_RELATIVE_PATH)
    if not state.exists or not state.managed or not state.integrity_ok:
        raise GitHubCliError("Primero instala un workflow válido con `chxchx-tech deploy setup`.")
    try:
        plan_deploy_workflow(root)
    except ValueError as exc:
        raise GitHubCliError(str(exc)) from exc
    if not confirmed and not dry_run:
        raise GitHubCliError("CD requiere confirmación explícita; revisa y vuelve a ejecutar con `--confirm`.")
    return dispatch_workflow(
        root,
        WORKFLOW_RELATIVE_PATH.name,
        ref,
        fields={"confirm_deploy": "true"} if confirmed else {},
        dry_run=dry_run,
    )


def deployment_status_payload(root: Path) -> dict[str, object]:
    project = root.resolve()
    state = current_workflow_state(project, WORKFLOW_RELATIVE_PATH)
    try:
        plan = plan_deploy_workflow(project)
        compose_file = plan.compose_file
        warnings: list[str] = []
    except ValueError as exc:
        compose_file = ""
        warnings = [str(exc)]

    auto_deploy_branch = None
    if state.exists and state.managed and state.integrity_ok:
        content = state.path.read_text(encoding="utf-8")
        if "  push:\n" in content:
            match = re.search(r"(?m)^      - '([^']+)'$", content)
            auto_deploy_branch = match.group(1) if match else None

    is_authenticated = False
    runs: list[dict[str, object]] = []
    message = ""
    if state.exists and state.managed and state.integrity_ok:
        try:
            is_authenticated = authenticated(project)
            if is_authenticated:
                runs = recent_runs(project, workflow=WORKFLOW_RELATIVE_PATH.name)
            else:
                message = "Inicia sesión con `gh auth login` para consultar o despachar despliegues."
        except GitHubCliError as exc:
            message = str(exc)
    elif not state.exists:
        message = "El CD está desactivado. Previsualiza y confirma `deploy setup` para habilitarlo."
    elif not state.managed or not state.integrity_ok:
        message = "El workflow de despliegue requiere revisión: no pertenece a ChxChx o cambió externamente."

    return {
        "schema": DEPLOY_STATUS_SCHEMA,
        "schema_version": 1,
        "project": str(project),
        "configured": state.exists,
        "managed": state.managed,
        "integrity_ok": state.integrity_ok,
        "workflow_path": str(state.path),
        "runtime": "Docker Compose" if compose_file else "",
        "compose_file": compose_file,
        "auto_deploy_branch": auto_deploy_branch,
        "required_variables": ["VPS_SSH_HOST", "VPS_SSH_USER", "VPS_DEPLOY_PATH"],
        "required_secrets": ["VPS_SSH_PRIVATE_KEY", "VPS_SSH_KNOWN_HOSTS"],
        "authenticated": is_authenticated,
        "recent_runs": runs,
        "warnings": warnings,
        "message": message,
    }
