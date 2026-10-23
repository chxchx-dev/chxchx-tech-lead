"""Safe ownership, backup, and idempotent writes for ChxChx workflows."""

from __future__ import annotations

import hashlib
import os
import re
import shutil
import tempfile
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Protocol

from ..core.paths import ensure_home
from .ci_workflows import WORKFLOW_RELATIVE_PATH

MANAGED_MARKER = "# chxchx-tech-managed: v1"
HASH_PREFIX = "# chxchx-tech-sha256: "


@dataclass(frozen=True, slots=True)
class WorkflowState:
    path: Path
    exists: bool
    managed: bool
    integrity_ok: bool


@dataclass(frozen=True, slots=True)
class WorkflowWriteResult:
    path: Path
    changed: bool
    backup: Path | None
    content: str


class WorkflowPlan(Protocol):
    path: Path
    content: str


def workflow_path(root: Path, relative_path: Path = WORKFLOW_RELATIVE_PATH) -> Path:
    project = root.resolve()
    if relative_path.is_absolute() or ".." in relative_path.parts:
        raise ValueError("La ruta del workflow no es relativa al proyecto.")
    candidate = project / relative_path
    if candidate.is_symlink() or not _inside_root(project, candidate):
        raise ValueError("La ruta de workflow apunta fuera del proyecto; no la modificaré.")
    return candidate


def current_workflow_state(
    root: Path, relative_path: Path = WORKFLOW_RELATIVE_PATH
) -> WorkflowState:
    path = workflow_path(root, relative_path)
    if not path.exists():
        return WorkflowState(path, False, False, False)
    if not path.is_file():
        raise ValueError("La ruta del workflow existe y no es un archivo regular.")
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise ValueError(f"No pude leer {path}: {exc}") from exc
    _, managed, valid = _managed_body(content)
    return WorkflowState(path, True, managed, valid)


def write_workflow(root: Path, plan: WorkflowPlan, *, dry_run: bool = False) -> WorkflowWriteResult:
    path = workflow_path(root, plan.path)
    body = plan.content.rstrip() + "\n"
    content = _with_marker(body)
    old_state = current_workflow_state(root, plan.path)
    if old_state.exists and not old_state.managed:
        raise ValueError(f"{path} ya existe y no está administrado por ChxChx; lo conservaré.")
    if old_state.exists and not old_state.integrity_ok:
        raise ValueError(f"{path} cambió fuera de ChxChx; revisa o respáldalo antes de regenerar.")
    if old_state.exists and path.read_text(encoding="utf-8") == content:
        return WorkflowWriteResult(path, False, None, content)
    if dry_run:
        return WorkflowWriteResult(path, True, None, content)

    scope = plan.path.stem.removeprefix("chxchx-")
    backup = _backup_existing(root, path, scope) if old_state.exists else None
    _atomic_write(path, content)
    return WorkflowWriteResult(path, True, backup, content)


def remove_workflow(
    root: Path,
    relative_path: Path = WORKFLOW_RELATIVE_PATH,
    *,
    dry_run: bool = False,
) -> WorkflowWriteResult:
    state = current_workflow_state(root, relative_path)
    if not state.exists:
        return WorkflowWriteResult(state.path, False, None, "")
    if not state.managed or not state.integrity_ok:
        raise ValueError("El workflow no coincide con el archivo administrado actual; no lo borraré.")
    content = state.path.read_text(encoding="utf-8")
    if dry_run:
        return WorkflowWriteResult(state.path, True, None, content)
    scope = relative_path.stem.removeprefix("chxchx-")
    backup = _backup_existing(root, state.path, scope)
    state.path.unlink()
    return WorkflowWriteResult(state.path, True, backup, content)


def _with_marker(body: str) -> str:
    digest = hashlib.sha256(body.encode("utf-8")).hexdigest()
    return f"{MANAGED_MARKER}\n{HASH_PREFIX}{digest}\n{body}"


def _managed_body(content: str) -> tuple[str, bool, bool]:
    lines = content.splitlines(keepends=True)
    managed = bool(lines and lines[0].rstrip("\r\n") == MANAGED_MARKER)
    if not managed or len(lines) < 3 or not lines[1].startswith(HASH_PREFIX):
        return content, managed, False
    expected = lines[1][len(HASH_PREFIX):].strip()
    if not re.fullmatch(r"[0-9a-f]{64}", expected):
        return content, True, False
    body = "".join(lines[2:])
    actual = hashlib.sha256(body.encode("utf-8")).hexdigest()
    return body, True, actual == expected


def _backup_existing(root: Path, source: Path, scope: str) -> Path:
    project = root.resolve()
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    key = hashlib.sha256(str(project).encode("utf-8")).hexdigest()[:12]
    backup_root = ensure_home() / "backups" / f"{project.name}-{key}" / scope / stamp
    relative_path = source.relative_to(project)
    backup_path = backup_root / relative_path
    backup_root.mkdir(parents=True, exist_ok=False)
    try:
        backup_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, backup_path)
    except OSError:
        shutil.rmtree(backup_root, ignore_errors=True)
        raise
    return backup_path


def _atomic_write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not _inside_root(path.parents[2], path):
        raise ValueError("El directorio del workflow apunta fuera del proyecto.")
    descriptor, temporary = tempfile.mkstemp(prefix=".chxchx-workflow-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    except OSError:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


def _inside_root(root: Path, target: Path) -> bool:
    try:
        target.resolve().relative_to(root.resolve())
    except (OSError, ValueError):
        return False
    return True
