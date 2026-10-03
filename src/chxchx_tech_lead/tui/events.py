from __future__ import annotations

import asyncio
from typing import Callable

from textual import work
from textual.widgets import Button, DataTable, Input, Static

from ..core.detector import detect_project
from ..core.registry import resolve_project_reference
from ..workspace.service import WorkspaceService
from ..workspace.state import load_state


class WorkspaceEvents:
    def button_init_preview(self) -> None:
        self.action_init_preview()

    def button_init(self) -> None:
        self.action_init_project()

    def button_init_minimal(self) -> None:
        self.action_init_minimal()

    def button_install_tools(self) -> None:
        self.action_install_tools()

    def button_integrate(self) -> None:
        self.action_integrate_clients()

    def button_setup_all(self) -> None:
        self.action_setup_all()

    def button_doctor(self) -> None:
        self.action_run_doctor()

    def button_open(self) -> None:
        self.action_open_workspace()

    def button_attach(self) -> None:
        self.action_attach_workspace()

    def button_attach_agents(self) -> None:
        self.action_attach_agents_workspace()

    def button_terminal(self) -> None:
        self.action_open_terminal()

    def button_trust(self) -> None:
        self.action_trust_workspace()

    def button_start(self) -> None:
        self.action_start_project()

    def button_console_start(self) -> None:
        self.action_start_project()

    def button_console_stop(self) -> None:
        self.action_stop_project()

    def button_console_trust(self) -> None:
        self.action_trust_workspace()
        self._refresh_project_console()

    def button_console_refresh(self) -> None:
        self._refresh_project_console()

    def button_start_all(self) -> None:
        self.action_start_workspace_all()

    def button_stop(self) -> None:
        self.action_stop_workspace()

    def button_refresh(self) -> None:
        self.action_refresh()

    def button_agents(self) -> None:
        self.action_start_agents()

    def button_agent_start(self) -> None:
        self.action_start_agent_selected()

    def button_agent_new_chat(self) -> None:
        self.action_start_new_chat()

    def button_agent_attach(self) -> None:
        self.action_attach_agent_terminal()

    def button_agents_refresh(self) -> None:
        self._refresh_agents()

    def button_process_start(self) -> None:
        self.action_start_process()

    def button_process_stop(self) -> None:
        self.action_stop_process()

    def button_process_refresh(self) -> None:
        self.refresh_dashboard()

    def button_resources_refresh(self) -> None:
        self.refresh_dashboard()

    def button_resources_stop(self) -> None:
        self.action_stop_workspace()

    def button_projects_refresh(self) -> None:
        self._refresh_projects()

    def button_switch(self) -> None:
        self._switch_project()

    def button_handoff(self) -> None:
        self.action_write_handoff()

    def button_handoff_refresh(self) -> None:
        self._refresh_handoff()

    def button_memory_refresh(self) -> None:
        self._refresh_memory_history()

    def button_chat_refresh(self) -> None:
        self._refresh_conversations()

    def button_errors_refresh(self) -> None:
        self._refresh_errors()

    def chat_search_changed(self, _event: Input.Changed) -> None:
        self._debounce_panel_refresh("chat")

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

    def error_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        error = self._cached_errors.get(str(event.row_key.value))
        if error is None:
            return
        self._query("#error-detail", Static).update(
            f"{error.operation}\n"
            f"Proyecto: {error.project} · {error.project_path}\n"
            f"Fecha UTC: {error.occurred_at}\n"
            f"{'─' * 48}\n\n{error.message}"
        )

    def console_process_highlighted(self, event: DataTable.RowHighlighted) -> None:
        self._select_console_process(str(event.row_key.value))

    def memory_search_changed(self, _event: Input.Changed) -> None:
        self._debounce_panel_refresh("memory")

    def _debounce_panel_refresh(self, panel: str) -> None:
        attribute = "_chat_search_timer" if panel == "chat" else "_memory_search_timer"
        timer = getattr(self, attribute)
        if timer is not None:
            timer.stop()
        callback = self._refresh_conversations if panel == "chat" else self._refresh_memory_history
        setattr(self, attribute, self.set_timer(0.25, callback))

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

    def agent_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        agent_id = str(event.row_key.value)
        self._query("#agent-id", Input).value = agent_id

    def _switch_project(self) -> None:
        reference = self._query("#project-ref", Input).value.strip()
        self._perform(
            "Proyecto cambiado",
            lambda: self._resolve_and_start_project(reference),
            refresh=False,
            on_success=self._finish_project_switch,
        )

    @staticmethod
    def _resolve_and_start_project(reference: str):
        root = resolve_project_reference(reference)
        selected = detect_project(root)
        service = WorkspaceService(selected)
        service.start()
        return selected, service

    def _finish_project_switch(self, result) -> None:
        selected, service = result
        self.service = service
        self.project = selected
        self.sub_title = self.subtitle
        self._query("#project-ref", Input).value = str(selected.root)
        self._set_log(f"Proyecto activo: {selected.name}")
        self._show("overview")
        self.refresh_dashboard()
        self._refresh_projects()
        self._refresh_handoff()
        self._refresh_project_console()

    def _perform(
        self,
        success: str,
        action: Callable[[], object],
        *,
        refresh: bool = True,
        on_success: Callable[[object], None] | None = None,
    ) -> None:
        if (
            self._operation_pending
            or self._setup_pending
            or self._attach_pending
            or self._start_pending
        ):
            self.notify("Espera a que termine la acción actual", severity="warning")
            return
        self._operation_pending = True
        self._set_log(f"Ejecutando: {success}…")
        self.notify(f"Ejecutando: {success}…", severity="information")
        self._execute_operation(success, action, refresh, on_success)

    @work(group="workspace-action", exclusive=True)
    async def _execute_operation(
        self,
        success: str,
        action: Callable[[], object],
        refresh: bool,
        on_success: Callable[[object], None] | None,
    ) -> None:
        try:
            result = await asyncio.to_thread(action)
        except Exception as exc:
            self._finish_operation(success, None, exc, refresh, on_success)
            return
        self._finish_operation(success, result, None, refresh, on_success)

    def _finish_operation(
        self,
        success: str,
        result: object | None,
        error: Exception | None,
        refresh: bool,
        on_success: Callable[[object], None] | None,
    ) -> None:
        self._operation_pending = False
        if error is not None:
            message = f"Error: {type(error).__name__}: {error}"
            self._set_log(message)
            self.notify(str(error), severity="error")
            return
        messages = getattr(result, "messages", [])
        detail = f": {messages[-1]}" if messages else ""
        if "Workspace" in success:
            state = load_state(self.project.root)
            session = state.session_name or "sin sesión persistente"
            detail += f" · Estado {state.status.value} · sesión {session}"
            if state.status.value == "ACTIVE":
                detail += " · usa Trabajo > Proyecto > Terminales Zellij para entrar"
        self._set_log(f"{success}{detail}")
        self.notify(f"{success}{detail}", severity="information")
        if on_success is not None:
            on_success(result)
        if refresh:
            self.refresh_dashboard()

    # Refreshable views
