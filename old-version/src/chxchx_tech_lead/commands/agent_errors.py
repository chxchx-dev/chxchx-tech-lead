"""Shared local error recording for agent commands."""

from pathlib import Path

from ..workspace.error_cache import record_error


def _record_agent_error(path: Path, operation: str, message: str) -> None:
    try:
        record_error(
            project=path.name or str(path),
            project_path=path,
            operation=operation,
            message=message,
        )
    except OSError:
        pass
