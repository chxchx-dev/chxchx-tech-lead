from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

from ..adapters.agents.base_cli import CliAgentAdapter
from ..adapters.docker.compose import DockerComposeAdapter
from ..adapters.editor.sublime import SublimeAdapter
from ..adapters.terminal.base import TerminalWorkspaceAdapter
from ..adapters.terminal.subprocess import SubprocessAdapter
from ..adapters.terminal.zellij import ZellijAdapter
from ..core.detector import detect_project
from ..core.models import ProjectInfo
from ..core.registry import load_registry, set_last_project
from .agent_commands import agent_pane_command
from .agent_status import AgentRuntimeStatus
from .manager import WorkspaceInspection, WorkspaceManager
from .layouts import workspace_layout
from .process_manager import ProcessActionResult, ProcessManager, ProcessManagerError, ProcessStatus
from .state import load_state, set_workspace_status
from .models import WorkspaceStatus


def _session_already_exists(result) -> bool:
    detail = f"{result.stdout}\n{result.stderr}".lower()
    return "session already exists" in detail or "a session by the name" in detail and "exists" in detail


class WorkspaceOperationError(RuntimeError):
    """Error accionable de una operación de workspace."""


@dataclass(slots=True)
class WorkspaceAction:
    inspection: WorkspaceInspection
    messages: list[str] = field(default_factory=list)
    process_results: list[ProcessActionResult] = field(default_factory=list)
    session_created: bool = False


class WorkspaceService:
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
                if not _session_already_exists(result):
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
        if inspection.config is not None and inspection.config.docker.enabled:
            docker = DockerComposeAdapter(self.project.root, inspection.config.docker.compose_file)
            if inspection.config.docker.auto_start:
                result = docker.up(dry_run=dry_run)
                if result.returncode != 0:
                    raise WorkspaceOperationError(result.stderr or "No se pudo iniciar Docker Compose")
                action.messages.append("Docker Compose iniciado")
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
        if inspection.config is not None and inspection.config.docker.enabled:
            docker = DockerComposeAdapter(self.project.root, inspection.config.docker.compose_file)
            result = docker.stop(dry_run=dry_run)
            if result.returncode == 0 and not result.skipped:
                action.messages.append("Docker Compose detenido")
            elif result.returncode != 0 and not result.skipped:
                raise WorkspaceOperationError(result.stderr or "No se pudo detener Docker Compose")
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
                focused = terminal.focus_terminal_pane(session)
                if focused.returncode != 0:
                    raise WorkspaceOperationError(
                        focused.stderr or f"No pude enfocar la pane `terminal` de `{session}`"
                    )
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

    def start_agent(self, agent_id: str, dry_run: bool = False, new_chat: bool = False):
        inspection = self.inspect()
        self._require_trust(inspection)
        if inspection.config is None:
            raise WorkspaceOperationError("No hay configuración de workspace")
        config = next((item for item in inspection.config.agents if item.id == agent_id), None)
        if config is None:
            raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
        if not isinstance(config.command, list) or not config.command:
            raise WorkspaceOperationError(f"El agente `{agent_id}` debe usar un comando por argumentos")
        session = self._session_name(inspection)
        terminal = self._terminal_adapter(inspection)
        if not dry_run and not terminal.session_exists(session):
            result = terminal.create_session(session, self.project.root)
            if result.returncode != 0 and not _session_already_exists(result):
                raise WorkspaceOperationError(result.stderr or f"No pude crear la sesión `{session}`")
        try:
            pane_command = agent_pane_command(
                agent_id,
                config.command,
                label=inspection.config.header.label,
                logo=inspection.config.header.logo,
                new_chat=new_chat,
            )
        except ValueError as exc:
            raise WorkspaceOperationError(str(exc)) from exc
        adapter = CliAgentAdapter(
            agent_id,
            sys.executable,
            arguments=pane_command[1:],
            terminal=terminal,
        )
        direction = "right" if inspection.config.layout.orientation == "horizontal" else "down"
        result = adapter.start(
            self.project.root / config.cwd,
            session=session,
            dry_run=dry_run,
            direction=direction,
        )
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or f"No se pudo iniciar el agente `{agent_id}`")
        return result

    def attach_agent(self, agent_id: str, dry_run: bool = False):
        """Attach to the named Zellij pane for one configured agent."""
        inspection = self.inspect()
        if inspection.config is None:
            raise WorkspaceOperationError("No hay configuración de workspace")
        if not any(item.id == agent_id for item in inspection.config.agents):
            raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
        terminal = self._terminal_adapter(inspection)
        if not isinstance(terminal, ZellijAdapter):
            raise WorkspaceOperationError("Abrir la terminal del agente requiere Zellij")
        session = self._session_name(inspection)
        if not dry_run:
            self._require_active_zellij_session(terminal, session, operation="adjuntar")
            focused = terminal.focus_named_pane(session, agent_id)
            if focused.returncode != 0:
                raise WorkspaceOperationError(focused.stderr or f"No pude enfocar la terminal de `{agent_id}`")
        result = terminal.attach_session(session, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or f"No pude abrir la terminal de `{agent_id}`")
        return result

    def run_all(self, dry_run: bool = False, attach: bool = True, recreate: bool = False):
        """Run the standard workspace recipe: start, agents and optional attach."""
        if recreate:
            inspection = self.inspect()
            self._require_trust(inspection)
            terminal = self._terminal_adapter(inspection)
            session = self._session_name(inspection)
            if dry_run or terminal.session_exists(session):
                result = terminal.close_session(session, dry_run=dry_run)
                if result.returncode != 0 and not dry_run:
                    raise WorkspaceOperationError(result.stderr or f"No pude recrear `{session}`")
        action = self.start(dry_run=dry_run, include_agents=True)
        # Newly created sessions already contain the complete horizontal/vertical
        # layout. Dynamic panes remain the fallback for an existing session.
        agent_results = (
            self.start_agents(dry_run=dry_run)
            if action.inspection.config and action.inspection.config.agents and not action.session_created
            else []
        )
        attach_result = self.attach(dry_run=dry_run) if attach else None
        return action, agent_results, attach_result

    def start_agents(self, dry_run: bool = False, new_chat: bool = False) -> list[tuple[str, object]]:
        """Inicia todos los agentes declarados en la configuración del proyecto."""
        return self._start_agent_ids(
            [agent.id for agent in self._configured_agents()], dry_run=dry_run, new_chat=new_chat
        )

    def start_preset(self, preset_id: str, dry_run: bool = False) -> list[tuple[str, object]]:
        inspection = self.inspect()
        self._require_trust(inspection)
        if inspection.config is None:
            raise WorkspaceOperationError("No hay configuración de workspace")
        preset = next((item for item in inspection.config.presets if item.id == preset_id), None)
        if preset is None:
            raise WorkspaceOperationError(f"No existe el preset de agentes: {preset_id}")
        return self._start_agent_ids(list(preset.agents), dry_run=dry_run)

    def agent_statuses(self) -> list[AgentRuntimeStatus]:
        inspection = self.inspect()
        if inspection.config is None:
            return []
        session = self._session_name(inspection)
        terminal = self._terminal_adapter(inspection)
        session_state = "MISSING"
        if hasattr(terminal, "session_status"):
            session_state = str(terminal.session_status(session)).upper()
        elif terminal.session_exists(session):
            session_state = "ACTIVE"
        panes = ""
        if session_state == "ACTIVE" and hasattr(terminal, "list_panes"):
            pane_result = terminal.list_panes(session)
            panes = pane_result.stdout if pane_result.returncode == 0 else ""
        presets = {
            agent_id: preset.id
            for preset in inspection.config.presets
            for agent_id in preset.agents
        }
        statuses: list[AgentRuntimeStatus] = []
        for config in inspection.config.agents:
            command = config.command[0] if isinstance(config.command, list) else config.command
            adapter = CliAgentAdapter(config.id, command)
            pane_running = bool(
                panes
                and any(
                    config.id.lower() in line.lower() and "EXITED" not in line.upper()
                    for line in panes.splitlines()
                )
            )
            statuses.append(
                AgentRuntimeStatus(
                    id=config.id,
                    command=command,
                    cwd=str((self.project.root / config.cwd).resolve()),
                    available=adapter.available(),
                    version=adapter.version(),
                    session=session_state,
                    pane="RUNNING" if pane_running else "NOT_FOUND",
                    preset=presets.get(config.id, "-"),
                )
            )
        return statuses

    def _configured_agents(self):
        inspection = self.inspect()
        self._require_trust(inspection)
        if inspection.config is None or not inspection.config.agents:
            raise WorkspaceOperationError("No hay agentes configurados en .ai/chxchx-tech.toml")
        return inspection.config.agents

    def _start_agent_ids(
        self, agent_ids: list[str], dry_run: bool, new_chat: bool = False
    ) -> list[tuple[str, object]]:
        if not agent_ids:
            raise WorkspaceOperationError("El preset no contiene agentes configurados")
        results = [
            (agent_id, self.start_agent(agent_id, dry_run=dry_run, new_chat=new_chat))
            for agent_id in agent_ids
        ]
        if not dry_run:
            inspection = self.inspect()
            terminal = self._terminal_adapter(inspection)
            if isinstance(terminal, ZellijAdapter):
                # Adding a pane normally focuses the new agent. Restore the
                # terminal pane so the user can type immediately after attach.
                focused = terminal.focus_terminal_pane(self._session_name(inspection))
                if focused.returncode != 0:
                    raise WorkspaceOperationError(
                        focused.stderr or "No pude devolver el foco a la pane `terminal`"
                    )
        return results

    def _start_all_agents_legacy(self, dry_run: bool = False) -> list[tuple[str, object]]:
        """Compatibility helper retained for callers using the old internal name."""
        inspection = self.inspect()
        self._require_trust(inspection)
        if inspection.config is None or not inspection.config.agents:
            raise WorkspaceOperationError("No hay agentes configurados en .ai/chxchx-tech.toml")
        return [
            (agent.id, self.start_agent(agent.id, dry_run=dry_run))
            for agent in inspection.config.agents
        ]

    def _start_auto_processes(
        self,
        inspection: WorkspaceInspection,
        session: str,
        dry_run: bool,
        only_missing: bool = False,
    ) -> list[ProcessActionResult]:
        if not inspection.trusted:
            return []
        if inspection.config is None:
            return []
        manager = self._process_manager(inspection)
        results = []
        for config in inspection.config.processes:
            if not config.auto_start:
                continue
            if only_missing:
                current = next((item for item in manager.list() if item.id == config.id), None)
                if current is not None and current.status is ProcessStatus.RUNNING:
                    continue
            try:
                results.append(manager.start(config.id, dry_run=dry_run))
            except ProcessManagerError as exc:
                raise WorkspaceOperationError(str(exc)) from exc
        return results

    def _process_manager(self, inspection: WorkspaceInspection) -> ProcessManager:
        return ProcessManager(
            self.project.root,
            inspection.config.processes if inspection.config is not None else (),
            trusted=inspection.trusted,
        )

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
