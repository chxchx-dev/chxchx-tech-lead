from __future__ import annotations

from dataclasses import dataclass, field

from .manager import WorkspaceInspection
from .process_models import ProcessActionResult


class WorkspaceOperationError(RuntimeError):
    """Error accionable de una operación de workspace."""


@dataclass(slots=True)
class WorkspaceAction:
    inspection: WorkspaceInspection
    messages: list[str] = field(default_factory=list)
    process_results: list[ProcessActionResult] = field(default_factory=list)
    session_created: bool = False


def session_already_exists(result) -> bool:
    detail = f"{result.stdout}\n{result.stderr}".lower()
    return "session already exists" in detail or "a session by the name" in detail and "exists" in detail
