from __future__ import annotations

import json

from ..adapters.terminal.zellij import ZellijAdapter
from .service_models import WorkspaceOperationError


def has_named_tab(raw: str, wanted: str) -> bool:
    try:
        payload = json.loads(raw)
    except (TypeError, ValueError):
        return any(wanted.casefold() in line.casefold() for line in raw.splitlines())

    def walk(value):
        if isinstance(value, dict):
            name = value.get("name") or value.get("tab_name")
            if isinstance(name, str) and name.casefold() == wanted.casefold():
                return True
            return any(walk(child) for child in value.values())
        if isinstance(value, list):
            return any(walk(child) for child in value)
        return False

    return walk(payload)


def ensure_layout_tab(
    terminal: ZellijAdapter,
    session: str,
    name: str,
    layout: str,
    *,
    dry_run: bool = False,
) -> None:
    tabs = terminal.list_tabs(session, dry_run=dry_run)
    if has_named_tab(tabs.stdout, name):
        return
    result = terminal.add_layout_tab(session, layout, dry_run=dry_run)
    if result.returncode != 0:
        raise WorkspaceOperationError(result.stderr or f"No pude crear la pestaña {name}")
    if not dry_run and not has_named_tab(terminal.list_tabs(session).stdout, name):
        raise WorkspaceOperationError(f"Zellij no confirmó la pestaña {name}")
