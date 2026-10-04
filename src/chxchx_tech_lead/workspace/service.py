from __future__ import annotations

import re
from pathlib import Path

from ..adapters.editor.sublime import SublimeAdapter
from ..adapters.terminal.base import TerminalWorkspaceAdapter
from ..adapters.terminal.subprocess import SubprocessAdapter
from ..adapters.terminal.zellij import ZellijAdapter
from ..core.detector import detect_project
from ..core.models import ProjectInfo
from ..core.registry import load_registry, set_last_project
from .agent_operations import WorkspaceAgentOperations
from .manager import WorkspaceInspection, WorkspaceManager
from .layouts import workspace_layout
from .process_manager import ProcessActionResult, ProcessManagerError, ProcessStatus
from .process_operations import WorkspaceProcessOperations
from .state import load_state, set_workspace_status
from .models import WorkspaceStatus
from .service_models import WorkspaceAction, WorkspaceOperationError, session_already_exists
from .terminal_operations import WorkspaceTerminalOperations


class WorkspaceService(
    WorkspaceAgentOperations,
    WorkspaceProcessOperations,
    WorkspaceTerminalOperations,
):
    """Orquesta adapters sin permitir que la CLI ejecute comandos directamente."""

    def __init__(
        self,
        project: ProjectInfo,
        *,
        terminal: TerminalWorkspaceAdapter | None = None,
        editor: SublimeAdapter | None = None,
    ):
        self.project = project
        self._manager = WorkspaceManager(project)
        self._terminal = terminal
        self._editor = editor

    def inspect(self) -> WorkspaceInspection:
        inspection = self._manager.inspect()
        if inspection.error:
            raise WorkspaceOperationError(inspection.error)
        return inspection

    def open(
        self,
        dry_run: bool = False,
        attach: bool = False,
        start_auto: bool = True,
        include_agents: bool = False,
    ) -> WorkspaceAction:
        inspection = self.inspect()
        terminal = self._terminal_adapter(inspection)
        session = self._session_name(inspection)
        action = WorkspaceAction(inspection)
        exists = False if dry_run else terminal.session_exists(session)
        if not exists:
            layout = workspace_layout(
                self.project.root,
                inspection.config.header if inspection.config is not None else None,
                self.project.name,
                self.project.profile_name,
                inspection.config.layout.orientation if inspection.config is not None else "horizontal",
                inspection.config.agents if include_agents and inspection.config is not None else (),
            ) if inspection.config is not None else None
            result = terminal.create_session(session, self.project.root, dry_run=dry_run, layout=layout)
            if result.returncode != 0:
                if not session_already_exists(result):
                    raise WorkspaceOperationError(result.stderr or f"No pude crear la sesión `{session}`")
                action.messages.append(f"Sesión existente: {session}")
            else:
                action.session_created = True
                action.messages.append(result.stdout or f"Sesión preparada: {session}")
                if layout is not None:
                    action.messages.append("Header de workspace configurado")
        else:
            action.messages.append(f"Sesión existente: {session}")

        # A successful create command is not enough: a stale Zellij server or
        # wrapper can return zero without leaving a usable session. Fail before
        # persisting ACTIVE in that situation.
        if not dry_run:
            self._require_active_zellij_session(terminal, session)

        if start_auto and inspection.config is not None and inspection.config.auto_start:
            action.process_results.extend(self._start_auto_processes(inspection, session, dry_run))
        if inspection.config is not None and inspection.config.auto_open_editor:
            result = self._editor_adapter().open_project(self.project.root, dry_run=dry_run)
            if result.returncode != 0:
                action.messages.append(result.stderr or "No se pudo abrir Sublime")
            else:
                action.messages.append("Editor abierto")
        if attach:
            if isinstance(terminal, ZellijAdapter):
                result = terminal.attach_session(
                    session,
                    dry_run=dry_run,
                    focus_tab="Terminales",
                )
            else:
                result = terminal.attach_session(session, dry_run=dry_run)
            if result.returncode != 0:
                raise WorkspaceOperationError(result.stderr or f"No pude adjuntar a `{session}`")
        return action

    def start(self, dry_run: bool = False, include_agents: bool = False) -> WorkspaceAction:
        inspection = self.inspect()
        self._require_trust(inspection)
        suspended = self._suspend_other_active_workspaces(dry_run=dry_run)
        session = self._session_name(inspection)
        try:
            action = self.open(
                dry_run=dry_run,
                attach=False,
                start_auto=False,
                include_agents=include_agents,
            )
        except WorkspaceOperationError:
            set_workspace_status(
                self.project.root,
                WorkspaceStatus.ERROR,
                session_name=session,
                dry_run=dry_run,
            )
            raise
        action.messages[0:0] = suspended
        action.process_results.extend(self._start_auto_processes(inspection, session, dry_run, only_missing=True))
        set_workspace_status(
            self.project.root,
            WorkspaceStatus.ACTIVE,
            session_name=session,
            dry_run=dry_run,
        )
        set_last_project(self.project.root, dry_run=dry_run)
        return action

    def _suspend_other_active_workspaces(self, dry_run: bool) -> list[str]:
        messages: list[str] = []
        current = self.project.root.resolve()
        for project in load_registry().get("projects", []):
            raw_path = project.get("path")
            if not isinstance(raw_path, str):
                continue
            other_root = Path(raw_path).expanduser().resolve()
            if other_root == current or not other_root.is_dir():
                continue
            if load_state(other_root).status is not WorkspaceStatus.ACTIVE:
                continue
            WorkspaceService(detect_project(other_root)).suspend(dry_run=dry_run)
            messages.append(f"Workspace suspendido: {other_root}")
        return messages

    def stop(self, dry_run: bool = False) -> WorkspaceAction:
        inspection = self.inspect()
        action = WorkspaceAction(inspection)
        manager = self._process_manager(inspection)
        for record in manager.list():
            if record.status is ProcessStatus.RUNNING:
                try:
                    action.process_results.append(manager.stop(record.id, dry_run=dry_run))
                except ProcessManagerError as exc:
                    raise WorkspaceOperationError(str(exc)) from exc
        set_workspace_status(self.project.root, WorkspaceStatus.STOPPED, dry_run=dry_run)
        return action

    def suspend(self, dry_run: bool = False) -> WorkspaceAction:
        """Stop managed work while keeping the workspace recoverable."""
        action = self.stop(dry_run=dry_run)
        set_workspace_status(self.project.root, WorkspaceStatus.SUSPENDED, dry_run=dry_run)
        action.messages.append("Workspace suspendido")
        return action

    def resume(self, dry_run: bool = False) -> WorkspaceAction:
        action = self.start(dry_run=dry_run)
        action.messages.append("Workspace reanudado")
        return action

    def attach(self, dry_run: bool = False):
        inspection = self.inspect()
        session = self._session_name(inspection)
        terminal = self._terminal_adapter(inspection)
        if not dry_run:
            self._require_active_zellij_session(terminal, session, operation="adjuntar")
            if isinstance(terminal, ZellijAdapter):
                self._ensure_terminal_tab(inspection, terminal, session)
                tab = terminal.focus_tab(session, "Terminales")
                if tab.returncode != 0:
                    raise WorkspaceOperationError(tab.stderr or "No pude abrir la pestaña Terminales")
                focused = terminal.focus_terminal_pane(session)
                if focused.returncode != 0:
                    raise WorkspaceOperationError(
                        focused.stderr or f"No pude enfocar la pane `terminal` de `{session}`"
                    )
        if isinstance(terminal, ZellijAdapter):
            result = terminal.attach_session(
                session,
                dry_run=dry_run,
                focus_tab="Terminales",
            )
        else:
            result = terminal.attach_session(session, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or f"No pude adjuntar a `{session}`")
        return result

    def open_editor(self, dry_run: bool = False):
        self.inspect()
        result = self._editor_adapter().open_project(self.project.root, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or "No se pudo abrir Sublime")
        return result

    def start_process(self, process_id: str, dry_run: bool = False) -> ProcessActionResult:
        """Start one configured process through the workspace service."""
        inspection = self.inspect()
        try:
            return self._process_manager(inspection).start(process_id, dry_run=dry_run)
        except ProcessManagerError as exc:
            raise WorkspaceOperationError(str(exc)) from exc

    def stop_process(self, process_id: str, dry_run: bool = False) -> ProcessActionResult:
        """Stop one managed process through the workspace service."""
        inspection = self.inspect()
        try:
            return self._process_manager(inspection).stop(process_id, dry_run=dry_run)
        except ProcessManagerError as exc:
            raise WorkspaceOperationError(str(exc)) from exc

    def _terminal_adapter(self, inspection: WorkspaceInspection) -> TerminalWorkspaceAdapter:
        if self._terminal is not None:
            return self._terminal
        adapter_name = inspection.config.adapter if inspection.config is not None else "zellij"
        if adapter_name == "subprocess":
            self._terminal = SubprocessAdapter()
        elif adapter_name == "zellij":
            zellij = ZellijAdapter()
            self._terminal = zellij if zellij.available() else SubprocessAdapter()
        else:
            raise WorkspaceOperationError(f"Adapter de terminal no soportado: {adapter_name}")
        return self._terminal

    @staticmethod
    def _require_active_zellij_session(
        terminal: TerminalWorkspaceAdapter,
        session: str,
        *,
        operation: str = "crear",
    ) -> None:
        """Fail fast when Zellij did not leave the requested session usable."""
        if not isinstance(terminal, ZellijAdapter):
            return
        status = terminal.session_status(session)
        if status == "active":
            return
        if status == "exited":
            raise WorkspaceOperationError(
                f"No se pudo {operation} la sesión `{session}`: Zellij la reporta como EXITED. "
                f"Ejecuta `zellij delete-session --force {session}` y vuelve a iniciar el workspace."
            )
        raise WorkspaceOperationError(
            f"No se pudo {operation} la sesión `{session}`: Zellij no la reconoce. "
            "Comprueba `zellij list-sessions` y vuelve a ejecutar `Iniciar workspace`."
        )

    def _editor_adapter(self) -> SublimeAdapter:
        if self._editor is None:
            self._editor = SublimeAdapter()
        return self._editor

    def _session_name(self, inspection: WorkspaceInspection) -> str:
        raw = inspection.config.name if inspection.config is not None else self.project.root.name
        name = re.sub(r"[^A-Za-z0-9_.-]+", "-", raw).strip("-")
        return name or "workspace"

    @staticmethod
    def _require_trust(inspection: WorkspaceInspection) -> None:
        if not inspection.trusted:
            raise WorkspaceOperationError("El proyecto no es confiable; ejecuta `chxchx-tech workspace trust` antes de iniciar comandos")
