from __future__ import annotations

from typing import Callable

from textual.widgets import Input, TabbedContent

from ..core.detector import detect_project
from ..core.registry import resolve_project_reference
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
            getattr(self, f"action_{command_id}")()

    def _show(self, tab_id: str) -> None:
        self._query("#tabs", TabbedContent).active = tab_id

    def action_show_overview(self) -> None:
        self._show("overview")

    def action_show_projects(self) -> None:
        self._show("projects")
        self._refresh_projects()

    def action_show_agents(self) -> None:
        self._show("agents")
        self._refresh_agents()

    def action_show_processes(self) -> None:
        self._show("processes")
        self.refresh_dashboard()

    def action_show_resources(self) -> None:
        self._show("resources")
        self.refresh_dashboard()

    def action_show_handoff(self) -> None:
        self._show("handoff")
        self._refresh_handoff()

    def action_show_memory(self) -> None:
        self._show("memory")
        self._refresh_memory_history()

    def action_show_conversations(self) -> None:
        self._show("conversations")
        self._refresh_conversations()

    def action_show_brand(self) -> None:
        self._show("brand-tab")

    def action_refresh(self) -> None:
        self.refresh_dashboard()

    # Workspace actions

    def action_open_workspace(self) -> None:
        self._perform("Workspace preparado", lambda: self.service.open())

    def action_attach_workspace(self) -> None:
        if self._attach_pending:
            return
        self._attach_pending = True
        message = "Entrando a Zellij… Para volver al TUI: Ctrl+O y después D."
        self._set_log(message)
        self.notify(message, severity="information")
        # Let Textual paint the instruction before the interactive Zellij
        # process takes control of the terminal.
        self.set_timer(0.1, self._attach_workspace_now)

    def action_attach_agent_terminal(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Selecciona un agente de la tabla antes de abrir su terminal")
            self.notify("Falta seleccionar un agente", severity="warning")
            return
        if self._attach_pending:
            return
        self._attach_pending = True
        message = f"Abriendo la terminal de {agent_id}… Para volver a la TUI: Ctrl+O y después D."
        self._set_log(message)
        self.notify(message, severity="information")
        self.set_timer(0.1, lambda: self._attach_agent_terminal_now(agent_id))

    def _attach_agent_terminal_now(self, agent_id: str) -> None:
        self._attach_pending = False
        try:
            with self.suspend():
                self.service.attach_agent(agent_id)
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            self._restore_after_external_terminal()
            return
        message = f"Terminal de {agent_id} cerrada; regresaste a la TUI."
        self._set_log(message)
        self.notify(message, severity="information")
        self._restore_after_external_terminal()

    def _attach_workspace_now(self) -> None:
        self._attach_pending = False
        try:
            # Textual owns the terminal in raw mode. Suspend it while
            # Zellij owns the terminal, then let Textual restore its
            # screen and keyboard handling after detach.
            with self.suspend():
                self.service.attach()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            self._restore_after_external_terminal()
            return
        message = "Zellij finalizado; regresaste al TUI. La sesión sigue disponible."
        self._set_log(message)
        self.notify(message, severity="information")
        self._restore_after_external_terminal()

    def _restore_after_external_terminal(self) -> None:
        """Force a repaint/layout pass after Zellij returns the terminal."""
        self.refresh_dashboard()
        self.refresh(repaint=True, layout=True)
        # Some terminal drivers deliver the resume event before the first
        # repaint. A second pass prevents controls from appearing only
        # after a mouse hover.
        self.set_timer(0.05, lambda: self.refresh(repaint=True, layout=True))

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
        self._queue_workspace_start(
            "Workspace y agentes iniciados",
            lambda: self.service.run_all(attach=False),
        )

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
        self._perform("Agentes iniciados", lambda: self.service.start_agents(), refresh=False)
        self._refresh_agents()

    def action_start_agent_selected(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe un ID de agente, por ejemplo `codex` o `claude`")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._perform(
            f"Agente `{agent_id}` iniciado",
            lambda: self.service.start_agent(agent_id),
            refresh=False,
        )
        self._refresh_agents()

    def action_start_new_chat(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe codex o claude para iniciar un chat nuevo con contexto")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._perform(
            f"Chat nuevo de `{agent_id}` iniciado con contexto persistido",
            lambda: self.service.start_agent(agent_id, new_chat=True),
            refresh=False,
        )
        self._refresh_agents()

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
        try:
            inspection = self.service.inspect()
            update_handoff(
                self.project,
                inspection.state.status,
                self.service.agent_statuses(),
                summary=self._query("#handoff-summary", Input).value.strip(),
                pending=self._query("#handoff-pending", Input).value.strip(),
                validation=self._query("#handoff-validation", Input).value.strip(),
            )
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            return
        self._set_log("Handoff actualizado")
        self._refresh_handoff()


