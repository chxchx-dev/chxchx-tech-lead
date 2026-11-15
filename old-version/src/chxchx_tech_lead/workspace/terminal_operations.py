from __future__ import annotations

from ..adapters.terminal.zellij import ZellijAdapter
from .layouts import terminal_tab_layout
from .service_models import WorkspaceOperationError
from .zellij_tabs import ensure_layout_tab


class WorkspaceTerminalOperations:
    """Use cases for terminal panes and tab attachment."""

    def open_terminal(self, dry_run: bool = False):
        """Create a parallel interactive terminal in the project's Zellij session."""
        inspection = self.inspect()
        terminal = self._terminal_adapter(inspection)
        if not isinstance(terminal, ZellijAdapter):
            raise WorkspaceOperationError("Abrir terminales paralelas requiere Zellij.")
        session = self._session_name(inspection)
        if not dry_run:
            self._require_active_zellij_session(terminal, session, operation="abrir terminal en")
        self._ensure_terminal_tab(inspection, terminal, session, dry_run=dry_run)
        result = terminal.open_terminal_pane(session, self.project.root, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or f"No pude abrir una terminal en `{session}`")
        if dry_run:
            return result
        attached = terminal.attach_session(session, focus_tab="Terminales")
        if attached.returncode != 0:
            raise WorkspaceOperationError(attached.stderr or f"No pude adjuntar a `{session}`")
        return attached

    def _ensure_terminal_tab(self, inspection, terminal, session: str, *, dry_run: bool = False) -> None:
        if not isinstance(terminal, ZellijAdapter) or inspection.config is None:
            return
        layout = terminal_tab_layout(
            self.project.root,
            inspection.config.header,
            self.project.name,
            self.project.profile_name,
        )
        ensure_layout_tab(terminal, session, "Terminales", layout, dry_run=dry_run)
