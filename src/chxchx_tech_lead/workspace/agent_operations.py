from __future__ import annotations

import sys

from ..adapters.agents.base_cli import CliAgentAdapter
from ..adapters.terminal.zellij import ZellijAdapter
from ..core.runner import CommandResult
from .agent_commands import agent_pane_command
from .agent_status import AgentRuntimeStatus
from .layouts import agents_tab_layout
from .manager import WorkspaceInspection
from .service_models import WorkspaceOperationError, session_already_exists
from .zellij_tabs import ensure_layout_tab, has_named_tab


class WorkspaceAgentOperations:
    """Casos de uso para iniciar, adjuntar y consultar agentes del workspace."""

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
            if result.returncode != 0 and not session_already_exists(result):
                raise WorkspaceOperationError(result.stderr or f"No pude crear la sesión `{session}`")
        self._ensure_agents_tab(inspection, terminal, session, dry_run=dry_run)
        if isinstance(terminal, ZellijAdapter):
            focused_tab = terminal.focus_tab(session, "Agentes", dry_run=dry_run)
            if focused_tab.returncode != 0:
                raise WorkspaceOperationError(focused_tab.stderr or "No pude abrir la pestaña Agentes")
            panes = terminal.list_panes(session, dry_run=dry_run)
            if self._pane_is_active(panes.stdout, agent_id) and not new_chat and not dry_run:
                return CommandResult(panes.command, 0, f"Agente `{agent_id}` ya está en la pestaña Agentes", "", skipped=True)
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
        """Attach to the agents tab and focus the configured agent pane."""
        inspection = self.inspect()
        if inspection.config is None:
            raise WorkspaceOperationError("No hay configuración de workspace")
        if not any(item.id == agent_id for item in inspection.config.agents):
            raise WorkspaceOperationError(f"No existe el agente configurado: {agent_id}")
        terminal = self._terminal_adapter(inspection)
        if not isinstance(terminal, ZellijAdapter):
            raise WorkspaceOperationError("Abrir la terminal del agente requiere Zellij")
        session = self._session_name(inspection)
        if not dry_run and not terminal.session_exists(session):
            self.start(include_agents=True)
        if not dry_run:
            self._require_active_zellij_session(terminal, session, operation="adjuntar")
        layout = self._agents_tab_layout(inspection)
        if isinstance(terminal, ZellijAdapter):
            result = terminal.attach_session(
                session,
                dry_run=dry_run,
                focus_tab="Agentes",
                focus_pane=agent_id,
                tab_layout=layout,
            )
        else:
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
        action, agent_results = self.prepare_agents(dry_run=dry_run)
        attach_result = (
            self.attach_agents(dry_run=dry_run, prepared=True)
            if attach and action.inspection.config and action.inspection.config.agents
            else self.attach(dry_run=dry_run) if attach else None
        )
        return action, agent_results, attach_result

    def prepare_agents(self, dry_run: bool = False):
        """Prepare the workspace; agent panes load once the client opens their tab."""
        action = self.start(dry_run=dry_run, include_agents=True)
        return action, []

    def attach_agents(self, dry_run: bool = False, prepared: bool = False):
        inspection = self.inspect()
        if inspection.config is None or not inspection.config.agents:
            raise WorkspaceOperationError("No hay agentes configurados")
        terminal = self._terminal_adapter(inspection)
        if not isinstance(terminal, ZellijAdapter) and not dry_run:
            raise WorkspaceOperationError("Adjuntar a los agentes requiere Zellij")
        session = self._session_name(inspection)
        self._require_active_zellij_session(terminal, session, operation="adjuntar a agentes") if not dry_run else None
        if isinstance(terminal, ZellijAdapter):
            result = terminal.attach_session(
                session,
                dry_run=dry_run,
                focus_tab="Agentes",
                tab_layout=self._agents_tab_layout(inspection),
            )
        else:
            result = terminal.attach_session(session, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or f"No pude adjuntar a `{session}`")
        return result

    def _ensure_agents_tab(self, inspection, terminal, session: str, *, dry_run: bool) -> None:
        if not isinstance(terminal, ZellijAdapter) or inspection.config is None:
            return
        tabs = terminal.list_tabs(session, dry_run=dry_run)
        if has_named_tab(tabs.stdout, "Agentes"):
            return
        layout = self._agents_tab_layout(inspection)
        ensure_layout_tab(terminal, session, "Agentes", layout, dry_run=dry_run)

    def _agents_tab_layout(self, inspection) -> str:
        return agents_tab_layout(
            self.project.root,
            inspection.config.header,
            self.project.name,
            self.project.profile_name,
            inspection.config.layout.orientation,
            inspection.config.agents,
        )

    @staticmethod
    def _pane_is_active(raw: str, agent_id: str) -> bool:
        matches = [line for line in raw.splitlines() if agent_id.casefold() in line.casefold()]
        return any("EXITED" not in line.upper() for line in matches)

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

    def agent_statuses(self, *, probe_versions: bool = True) -> list[AgentRuntimeStatus]:
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
                    version=adapter.version() if probe_versions else None,
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
                focused = terminal.focus_tab(self._session_name(inspection), "Agentes")
                if focused.returncode != 0:
                    raise WorkspaceOperationError(
                        focused.stderr or "No pude devolver el foco a la pestaña Agentes"
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
