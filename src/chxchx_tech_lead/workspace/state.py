from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from ..core.paths import ensure_home, home_dir
from .models import WorkspaceStatus


def state_path() -> Path:
    return home_dir() / "workspace-state.json"


@dataclass(slots=True)
class WorkspaceState:
    project_path: str
    status: WorkspaceStatus = WorkspaceStatus.STOPPED
    session_name: str | None = None
    process_ids: list[str] = field(default_factory=list)
    processes: dict[str, dict[str, Any]] = field(default_factory=dict)
    updated_at: str | None = None

    def touch(self) -> None:
        self.updated_at = datetime.now(timezone.utc).isoformat()


def _state_from_mapping(raw: Any) -> WorkspaceState | None:
    if not isinstance(raw, dict) or not isinstance(raw.get("project_path"), str):
        return None
    try:
        status = WorkspaceStatus(raw.get("status", WorkspaceStatus.STOPPED))
    except ValueError:
        status = WorkspaceStatus.ERROR
    process_ids = raw.get("process_ids", [])
    if not isinstance(process_ids, list) or not all(isinstance(item, str) for item in process_ids):
        process_ids = []
    processes = raw.get("processes", {})
    if not isinstance(processes, dict):
        processes = {}
    return WorkspaceState(
        project_path=raw["project_path"],
        status=status,
        session_name=raw.get("session_name") if isinstance(raw.get("session_name"), str) else None,
        process_ids=process_ids,
        processes={key: value for key, value in processes.items() if isinstance(key, str) and isinstance(value, dict)},
        updated_at=raw.get("updated_at") if isinstance(raw.get("updated_at"), str) else None,
    )


def load_states() -> dict[str, WorkspaceState]:
    path = state_path()
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    raw_states = data.get("workspaces", {}) if isinstance(data, dict) else {}
    if not isinstance(raw_states, dict):
        return {}
    result: dict[str, WorkspaceState] = {}
    for key, value in raw_states.items():
        state = _state_from_mapping(value)
        if state is not None:
            result[key] = state
    return result


def load_state(project_root: Path) -> WorkspaceState:
    root = project_root.resolve()
    return load_states().get(str(root), WorkspaceState(project_path=str(root)))


def save_state(state: WorkspaceState, dry_run: bool = False) -> None:
    state.touch()
    states = load_states()
    states[state.project_path] = state
    if not dry_run:
        ensure_home()
        payload = {"workspaces": {key: asdict(value) for key, value in states.items()}}
        payload["workspaces"][state.project_path]["status"] = state.status.value
        state_path().write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def set_workspace_status(
    project_root: Path,
    status: WorkspaceStatus,
    *,
    session_name: str | None = None,
    dry_run: bool = False,
) -> WorkspaceState:
    """Persist the lifecycle state without requiring a running process."""
    state = load_state(project_root)
    state.status = status
    if session_name is not None:
        state.session_name = session_name
    save_state(state, dry_run=dry_run)
    return state
