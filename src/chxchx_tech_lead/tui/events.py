from __future__ import annotations

from typing import Callable

from textual import on
from textual.widgets import Button, DataTable, Input, Static

from ..core.detector import detect_project
from ..core.registry import resolve_project_reference
from ..workspace.service import WorkspaceOperationError, WorkspaceService
from ..workspace.state import load_state


class WorkspaceEvents:
    @on(Button.Pressed, "#btn-open")
    def button_open(self) -> None:
        self.action_open_workspace()

    @on(Button.Pressed, "#btn-attach")
    def button_attach(self) -> None:
        self.action_attach_workspace()

    @on(Button.Pressed, "#btn-trust")
    def button_trust(self) -> None:
        self.action_trust_workspace()

    @on(Button.Pressed, "#btn-start")
    def button_start(self) -> None:
        self.action_start_project()

    @on(Button.Pressed, "#btn-console-start")
    def button_console_start(self) -> None:
        self.action_start_project()

    @on(Button.Pressed, "#btn-console-stop")
    def button_console_stop(self) -> None:
        self.action_stop_project()

    @on(Button.Pressed, "#btn-console-trust")
    def button_console_trust(self) -> None:
        self.action_trust_workspace()
        self._refresh_project_console()

    @on(Button.Pressed, "#btn-console-refresh")
    def button_console_refresh(self) -> None:
        self._refresh_project_console()

    @on(Button.Pressed, "#btn-start-all")
    def button_start_all(self) -> None:
        self.action_start_workspace_all()

    @on(Button.Pressed, "#btn-stop")
    def button_stop(self) -> None:
        self.action_stop_workspace()

    @on(Button.Pressed, "#btn-refresh")
    def button_refresh(self) -> None:
        self.action_refresh()

    @on(Button.Pressed, "#btn-agents")
    def button_agents(self) -> None:
        self.action_start_agents()

    @on(Button.Pressed, "#btn-agent-start")
    def button_agent_start(self) -> None:
        self.action_start_agent_selected()

    @on(Button.Pressed, "#btn-agent-new-chat")
    def button_agent_new_chat(self) -> None:
        self.action_start_new_chat()

    @on(Button.Pressed, "#btn-agent-attach")
    def button_agent_attach(self) -> None:
        self.action_attach_agent_terminal()

    @on(Button.Pressed, "#btn-agents-refresh")
    def button_agents_refresh(self) -> None:
        self._refresh_agents()

    @on(Button.Pressed, "#btn-process-start")
    def button_process_start(self) -> None:
        self.action_start_process()

    @on(Button.Pressed, "#btn-process-stop")
    def button_process_stop(self) -> None:
        self.action_stop_process()

    @on(Button.Pressed, "#btn-process-refresh")
    def button_process_refresh(self) -> None:
        self.refresh_dashboard()

    @on(Button.Pressed, "#btn-resources-refresh")
    def button_resources_refresh(self) -> None:
        self.refresh_dashboard()

    @on(Button.Pressed, "#btn-resources-stop")
    def button_resources_stop(self) -> None:
        self.action_stop_workspace()

    @on(Button.Pressed, "#btn-projects-refresh")
    def button_projects_refresh(self) -> None:
        self._refresh_projects()

    @on(Button.Pressed, "#btn-switch")
    def button_switch(self) -> None:
        self._switch_project()

    @on(Button.Pressed, "#btn-handoff")
    def button_handoff(self) -> None:
        self.action_write_handoff()

    @on(Button.Pressed, "#btn-handoff-refresh")
    def button_handoff_refresh(self) -> None:
        self._refresh_handoff()

    @on(Button.Pressed, "#btn-memory-refresh")
    def button_memory_refresh(self) -> None:
        self._refresh_memory_history()

    @on(Button.Pressed, "#btn-chat-refresh")
    def button_chat_refresh(self) -> None:
        self._refresh_conversations()

    @on(Input.Changed, "#chat-search")
    def chat_search_changed(self, _event: Input.Changed) -> None:
        self._refresh_conversations()

    @on(DataTable.RowHighlighted, "#chat-list")
    def chat_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        conversation = self._conversations.get(str(event.row_key.value))
        if conversation is None:
            return
        detail = [
            f"{conversation.title}",
            f"{conversation.provider} · {conversation.modified_at.strftime('%Y-%m-%d %H:%M %Z')}",
            f"Sesión: {conversation.session_id}",
            "─" * 48,
            "",
        ]
        detail.extend(
            f"{'Tú' if item.role == 'user' else conversation.provider}:\n{item.text}"
            for item in conversation.messages
        )
        self._query("#chat-detail", Static).update("\n\n".join(detail))

    @on(DataTable.RowHighlighted, "#console-processes")
    def console_process_highlighted(self, event: DataTable.RowHighlighted) -> None:
        self._select_console_process(str(event.row_key.value))

    @on(Input.Changed, "#memory-search")
    def memory_search_changed(self, _event: Input.Changed) -> None:
        self._refresh_memory_history()

    @on(DataTable.RowHighlighted, "#memory-list")
    def memory_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        note = self._memory_notes.get(str(event.row_key.value))
        if note is None:
            return
        relative_path = note.path.relative_to(self.project.root)
        detail = (
            f"{note.title}\n"
            f"Ruta: {relative_path}\n"
            f"Modificada: {note.modified_at.strftime('%Y-%m-%d %H:%M %Z')}\n"
            f"{'─' * 40}\n\n"
            f"{note.content}"
        )
        self._query("#memory-detail", Static).update(detail)

    @on(DataTable.RowHighlighted, "#agents-table")
    def agent_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        agent_id = str(event.row_key.value)
        self._query("#agent-id", Input).value = agent_id
        self._set_log(f"Agente seleccionado: {agent_id} · inicia o abre su terminal aquí")

    def _switch_project(self) -> None:
        reference = self._query("#project-ref", Input).value.strip()
        try:
            root = resolve_project_reference(reference)
            selected = detect_project(root)
            self.service = WorkspaceService(selected)
            self.service.start()
        except (ValueError, WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error al cambiar proyecto: {exc}")
            self.notify(str(exc), severity="error")
            return
        self.project = selected
        self.sub_title = self.subtitle
        self._query("#project-ref", Input).value = str(selected.root)
        self._set_log(f"Proyecto activo: {selected.name}")
        self.refresh_dashboard()
        self._refresh_projects()
        self._refresh_handoff()
        self._refresh_memory_history()
        self._refresh_conversations()
        self._refresh_project_console()

    def _perform(
        self,
        success: str,
        action: Callable[[], object],
        *,
        refresh: bool = True,
    ) -> None:
        try:
            result = action()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            return
        messages = getattr(result, "messages", [])
        detail = f": {messages[-1]}" if messages else ""
        if "Workspace" in success:
            state = load_state(self.project.root)
            session = state.session_name or "sin sesión persistente"
            detail += f" · Estado {state.status.value} · sesión {session}"
            if state.status.value == "ACTIVE":
                detail += " · usa `4. Adjuntar Zellij` para entrar"
        self._set_log(f"{success}{detail}")
        self.notify(f"{success}{detail}", severity="information")
        if refresh:
            self.refresh_dashboard()

    # Refreshable views
