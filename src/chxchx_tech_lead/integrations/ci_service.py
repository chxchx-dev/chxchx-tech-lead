"""Use cases for optional project CI workflows and GitHub run visibility."""

from __future__ import annotations

from pathlib import Path

from ..core.detector import detect_project
from ..core.runner import CommandResult
from .ci_workflow_files import (
    WorkflowWriteResult,
    current_workflow_state,
    remove_workflow,
    write_workflow,
)
from .ci_workflows import CiWorkflowPlan, plan_ci_workflow
from .github_cli import GitHubCliError, authenticated, dispatch_ci, failed_logs, recent_runs

CI_STATUS_SCHEMA = "chxchx.ci-status"


def setup_ci(root: Path, *, dry_run: bool = False) -> tuple[WorkflowWriteResult, CiWorkflowPlan]:
    plan = plan_ci_workflow(root)
    result = write_workflow(root, plan, dry_run=dry_run)
    return result, plan


def remove_ci(root: Path, *, dry_run: bool = False) -> WorkflowWriteResult:
    return remove_workflow(root, dry_run=dry_run)


def run_ci(root: Path, ref: str | None = None, *, dry_run: bool = False) -> CommandResult:
    state = current_workflow_state(root)
    if not state.exists or not state.managed or not state.integrity_ok:
        raise GitHubCliError("Primero instala un workflow válido con `chxchx-tech ci setup`.")
    return dispatch_ci(root, ref, dry_run=dry_run)


def ci_logs(root: Path, run_id: str) -> CommandResult:
    return failed_logs(root, run_id)


def ci_status_payload(root: Path) -> dict[str, object]:
    project = root.resolve()
    detected = detect_project(project)
    state = current_workflow_state(project)
    try:
        plan = plan_ci_workflow(project)
        supported = list(plan.stacks)
        warnings = list(plan.warnings)
    except ValueError as exc:
        supported = list(detected.stacks)
        warnings = [str(exc)]

    is_authenticated = False
    runs: list[dict[str, object]] = []
    message = ""
    if state.exists and state.managed and state.integrity_ok:
        try:
            is_authenticated = authenticated(project)
            if is_authenticated:
                runs = recent_runs(project)
            else:
                message = "Inicia sesión con `gh auth login` para ver y ejecutar flujos de GitHub."
        except GitHubCliError as exc:
            message = str(exc)
    elif not state.exists:
        message = "CI/CD está desactivado en este proyecto. Previsualiza y confirma `ci setup` para habilitar CI."
    elif not state.managed or not state.integrity_ok:
        message = "El workflow requiere revisión porque no es administrado por ChxChx o su integridad cambió."

    return {
        "schema": CI_STATUS_SCHEMA,
        "schema_version": 1,
        "project": str(project),
        "configured": state.exists,
        "managed": state.managed,
        "integrity_ok": state.integrity_ok,
        "workflow_path": str(state.path),
        "supported_stacks": supported,
        "warnings": warnings,
        "authenticated": is_authenticated,
        "recent_runs": runs,
        "message": message,
    }
