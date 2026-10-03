from __future__ import annotations

import asyncio
from typing import Callable

from textual import work
from textual.widgets import Input, TabbedContent

from ..core.trust import trust_project
from ..workspace.handoff import update_handoff
from ..workspace.service import WorkspaceOperationError
from ..workspace.state import load_state
from .palette import CommandPalette


class WorkspaceActions:
    def action_command_palette(self) -> None:
        self._palette_open = True
        self.push_screen(CommandPalette(), self._run_palette_command)

    def _run_palette_command(self, command_id: str | None) -> None:
        self._palette_open = False
        if command_id:
            handler = getattr(self, f"action_{command_id}", None)
            if handler is None:
                self.notify("No reconozco esa acción", severity="error")
            else:
                handler()
        if self._dashboard_refresh_again and not self._dashboard_pending:
            self.refresh_dashboard()

    def _show(self, tab_id: str) -> None:
        nested_tabs = {
            "console": ("work", "work-tabs"),
            "agents": ("work", "work-tabs"),
            "processes": ("work", "work-tabs"),
            "resources": ("more", "more-tabs"),
            "handoff": ("more", "more-tabs"),
            "memory": ("more", "more-tabs"),
            "conversations": ("more", "more-tabs"),
            "errors": ("more", "more-tabs"),
            "guide": ("more", "more-tabs"),
            "setup": ("more", "more-tabs"),
            "brand-tab": ("more", "more-tabs"),
        }
        destination = nested_tabs.get(tab_id)
        if destination:
            parent, child_tabs = destination
            self._query("#tabs", TabbedContent).active = parent
            self._query(f"#{child_tabs}", TabbedContent).active = tab_id
            return
        self._query("#tabs", TabbedContent).active = tab_id

    def action_show_overview(self) -> None:
        self._show("overview")

    def action_show_work(self) -> None:
        self._show("work")

    def action_show_more(self) -> None:
        self._show("more")

    def action_show_guide(self) -> None:
        self._show("guide")

    def action_show_setup(self) -> None:
        self._show("setup")

    def action_show_projects(self) -> None:
        self._show("projects")

    def action_show_agents(self) -> None:
        self._show("agents")

    def action_show_processes(self) -> None:
        self._show("processes")

    def action_show_resources(self) -> None:
        self._show("resources")

    def action_show_handoff(self) -> None:
        self._show("handoff")

    def action_show_memory(self) -> None:
        self._show("memory")

    def action_show_conversations(self) -> None:
        self._show("conversations")

    def action_show_errors(self) -> None:
        self._show("errors")

    def action_show_brand(self) -> None:
        self._show("brand-tab")

    def action_refresh(self) -> None:
        self.refresh_dashboard()

    # Workspace actions

    def action_open_workspace(self) -> None:
        self._perform("Workspace preparado", lambda: self.service.open())

    def action_trust_workspace(self) -> None:
        try:
            changed = trust_project(self.project.root)
        except OSError as exc:
            self._set_log(f"Error al confiar el proyecto: {exc}")
            self.notify(str(exc), severity="error")
            return
        message = "Proyecto marcado como confiable" if changed else "Proyecto ya era confiable"
        self._set_log(message)
        self.notify(message, severity="information")
        self.refresh_dashboard()

    def action_start_workspace(self) -> None:
        self._queue_workspace_start(
            "Workspace y agentes iniciados",
            lambda: self.service.run_all(attach=False),
        )

    def action_start_workspace_all(self) -> None:
        if self._attach_pending or self._start_pending or self._operation_pending or self._setup_pending:
            return
        self._attach_pending = True
        message = "Iniciando workspace y agentes… Zellij se abrirá al terminar."
        self._set_log(message)
        self.notify(message, severity="information")
        self.set_timer(0.1, self._launch_workspace_with_agents)

    def _launch_workspace_with_agents(self) -> None:
        try:
            # Prepara la sesión con la TUI visible. Solo cede el TTY al adjuntar.
            self._set_log("Preparando sesión y agentes; la TUI seguirá visible…")
            self._prepare_agents_for_attach()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            self._attach_pending = False
            self._refresh_agents()

    @work(group="workspace-agents", exclusive=True)
    async def _prepare_agents_for_attach(self) -> None:
        try:
            action, _agent_results = await asyncio.to_thread(self.service.prepare_agents)
        except Exception as exc:
            self._agents_prepare_failed(exc)
            return
        config = action.inspection.config
        self._attach_prepared_agents(bool(config and config.agents))

    def _agents_prepare_failed(self, exc: Exception) -> None:
        self._set_log(f"Error preparando agentes: {type(exc).__name__}: {exc}")
        self.notify(str(exc), severity="error")
        self._attach_pending = False
        self._refresh_agents()

    def _attach_prepared_agents(self, include_agents: bool = True) -> None:
        try:
            with self.suspend():
                if include_agents:
                    self.service.attach_agents(prepared=True)
                else:
                    self.service.attach()
        except Exception as exc:
            self._set_log(f"Error al abrir Zellij: {type(exc).__name__}: {exc}")
            self.notify(str(exc), severity="error")
        else:
            message = "Zellij cerrado; regresaste al TUI. Los agentes siguen en la sesión."
            self._set_log(message)
            self.notify(message, severity="information")
        finally:
            self._attach_pending = False
            self._restore_after_external_terminal()

    def _queue_workspace_start(self, success: str, action: Callable[[], object]) -> None:
        if self._start_pending:
            return
        self._start_pending = True
        message = "Iniciando workspace… espera la confirmación de estado ACTIVE."
        self._set_log(message)
        self.notify(message, severity="information")
        # Paint the progress message before the synchronous adapter calls
        # create the background session and validate it.
        self.set_timer(0.05, lambda: self._run_workspace_start(success, action))

    def _run_workspace_start(self, success: str, action: Callable[[], object]) -> None:
        self._start_pending = False
        self._perform(success, action)

    def action_stop_workspace(self) -> None:
        self._perform("Workspace detenido", lambda: self.service.stop())

    def action_suspend_workspace(self) -> None:
        self._perform("Workspace suspendido", lambda: self.service.suspend())

    def action_resume_workspace(self) -> None:
        self._perform("Workspace reanudado", lambda: self.service.resume())

    def action_start_agents(self) -> None:
        self._perform("Agentes iniciados", lambda: self.service.start_agents())

    def action_start_agent_selected(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe un ID de agente, por ejemplo `codex` o `claude`")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._perform(
            f"Agente `{agent_id}` iniciado",
            lambda: self.service.start_agent(agent_id),
        )

    def action_start_new_chat(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe codex o claude para iniciar un chat nuevo con contexto")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._perform(
            f"Chat nuevo de `{agent_id}` iniciado con contexto persistido",
            lambda: self.service.start_agent(agent_id, new_chat=True),
        )

    def _selected_process_action(self, *, start: bool) -> None:
        process_id = self._query("#process-id", Input).value.strip()
        if not process_id:
            self._set_log("Escribe un ID de proceso configurado")
            self.notify("Falta el ID del proceso", severity="warning")
            return
        action = self.service.start_process if start else self.service.stop_process
        verb = "iniciado" if start else "detenido"
        self._perform(
            f"Proceso `{process_id}` {verb}",
            lambda: action(process_id),
        )

    def action_start_process(self) -> None:
        self._selected_process_action(start=True)

    def action_stop_process(self) -> None:
        self._selected_process_action(start=False)

    def action_open_editor(self) -> None:
        self._perform("Editor abierto", lambda: self.service.open_editor())

    def action_write_handoff(self) -> None:
        summary = self._query("#handoff-summary", Input).value.strip()
        pending = self._query("#handoff-pending", Input).value.strip()
        validation = self._query("#handoff-validation", Input).value.strip()

        def write():
            inspection = self.service.inspect()
            return update_handoff(
                self.project,
                inspection.state.status,
                self.service.agent_statuses(),
                summary=summary,
                pending=pending,
                validation=validation,
            )

        self._perform(
            "Handoff actualizado",
            write,
            refresh=False,
            on_success=lambda _result: self._refresh_handoff(),
        )
