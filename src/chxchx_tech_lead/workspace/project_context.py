"""Resolve project references and compose their workspace service."""

from __future__ import annotations

from pathlib import Path

from ..core.detector import detect_project
from ..core.registry import resolve_project_reference
from .service import WorkspaceOperationError, WorkspaceService


def project_for_path(path: Path):
    return detect_project(path)


def resolve_project_path(reference: str | Path) -> Path:
    try:
        return resolve_project_reference(reference)
    except ValueError as exc:
        raise WorkspaceOperationError(str(exc)) from exc


def workspace_service_for(path: str | Path) -> WorkspaceService:
    return WorkspaceService(project_for_path(resolve_project_path(path)))
