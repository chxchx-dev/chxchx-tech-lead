from __future__ import annotations

from textual import on
from textual.app import App, ComposeResult, ScreenStackError
from textual.binding import Binding
from textual.widgets import (
    Button,
    DataTable,
    Input,
    Static,
    TabbedContent,
)

from ..core.models import ProjectInfo
from ..integrations.chat_history import Conversation
from ..workspace.agent_status import AgentRuntimeStatus
from ..workspace.error_cache import CachedError
from ..workspace.memory_history import MemoryNote
from ..workspace.service import WorkspaceService
from .actions import WorkspaceActions
from .dashboard import WorkspaceDashboard
from .events import WorkspaceEvents
from .layout import compose_workspace
from .panels import WorkspacePanels
from .palette import CommandPalette
from .project_console import WorkspaceProjectConsole
from .style import TUI_BINDINGS, TUI_CSS
from .setup_actions import WorkspaceSetupActions
from .terminal_actions import WorkspaceTerminalActions


class WorkspaceConsole(
    WorkspaceActions,
    WorkspaceSetupActions,
    WorkspaceTerminalActions,
    WorkspaceEvents,
    WorkspaceDashboard,
    WorkspacePanels,
    WorkspaceProjectConsole,
    App[None],
):
    TITLE = "ChxChx Terminal Workspace"
    CSS = TUI_CSS
    BINDINGS = [
        Binding(key, action, description, priority=key in {"ctrl+p", "f1", "f2"})
        for key, action, description in TUI_BINDINGS
    ]

    def __init__(self, project: ProjectInfo) -> None:
        super().__init__()
        self.project = project
        self.service = WorkspaceService(project)
        self._last_inspection = None
        self._dashboard_pending = False
        self._dashboard_refresh_again = False
        self._last_agents: list[AgentRuntimeStatus] = []
        self._memory_notes: dict[str, MemoryNote] = {}
        self._conversations: dict[str, Conversation] = {}
        self._cached_errors: dict[str, CachedError] = {}
        self._palette_open = False
        self._attach_pending = False
        self._start_pending = False
        self._operation_pending = False
        self._setup_pending = False
        self._console_process_id: str | None = None
        self._console_processes = {}
        self._console_pending = False
        self._console_refresh_again = False
        self._console_output_pending = False
        self._conversation_refresh_generation = 0
        self._memory_refresh_generation = 0
        self._chat_search_timer = None
        self._memory_search_timer = None
        self._tab_refresh_timer = None

    @property
    def subtitle(self) -> str:
        return f"{self.project.name} · {self.project.profile_name}"

    def compose(self) -> ComposeResult:
        yield from compose_workspace(self.project)

    def on_mount(self) -> None:
        self.sub_title = self.subtitle
        self._setup_tables()
        self._setup_panel_titles()
        self._refresh_project_console()
        self.set_interval(1, self._refresh_project_output)
        self.refresh_dashboard()

    def on_unmount(self) -> None:
        for timer in (
            self._tab_refresh_timer,
            self._chat_search_timer,
            self._memory_search_timer,
        ):
            if timer is not None:
                timer.stop()
        self._tab_refresh_timer = None
        self._chat_search_timer = None
        self._memory_search_timer = None

    @on(TabbedContent.TabActivated)
    def on_tab_activated(self, event: TabbedContent.TabActivated) -> None:
        if event.tabbed_content.id not in {"tabs", "work-tabs", "more-tabs"}:
            return
        if self._tab_refresh_timer is not None:
            self._tab_refresh_timer.stop()
        self._tab_refresh_timer = self.set_timer(0.05, self._refresh_visible_panel)

    def _refresh_visible_panel(self) -> None:
        self._tab_refresh_timer = None
        if self._palette_open:
            return
        try:
            section = self._query("#tabs", TabbedContent).active
        except ScreenStackError:
            return
        if section == "projects":
            self._refresh_projects()
        elif section == "work":
            if self._query("#work-tabs", TabbedContent).active == "console":
                self._refresh_project_console()
            else:
                self.refresh_dashboard()
        elif section == "more":
            active = self._query("#more-tabs", TabbedContent).active
            if active == "resources":
                self.refresh_dashboard()
            elif active == "handoff":
                self._refresh_handoff()
            elif active == "memory":
                self._refresh_memory_history()
            elif active == "conversations":
                self._refresh_conversations()
            elif active == "errors":
                self._refresh_errors()

    def _setup_tables(self) -> None:
        self._query("#overview-processes", DataTable).add_columns(
            "ID", "Estado", "PID", "Puerto", "RAM", "CPU"
        )
        self._query("#processes-table", DataTable).add_columns(
            "ID", "Estado", "PID", "Puerto", "RAM", "CPU"
        )
        self._query("#chat-list", DataTable).add_columns(
            "Agente", "Último uso", "Chat", "Último mensaje",
        )
        self._query("#projects-table", DataTable).add_columns(
            "Actual", "Alias", "Proyecto", "Estado", "Perfil", "Ruta"
        )
        self._query("#agents-table", DataTable).add_columns(
            "Agente", "CLI", "Disponible", "Sesión", "Pane", "Preset"
        )
        self._query("#resources-processes", DataTable).add_columns(
            "Proceso", "Estado", "PID", "RAM RSS", "CPU"
        )
        self._query("#resources-all-projects", DataTable).add_columns(
            "Actual", "Alias", "Proyecto", "Estado", "Activos", "RAM RSS", "CPU"
        )
        self._query("#memory-list", DataTable).add_columns(
            "Nota", "Modificada", "Resumen"
        )
        self._query("#console-processes", DataTable).add_columns(
            "ID", "Comando configurado", "Auto al abrir", "Estado"
        )
        self._query("#errors-table", DataTable).add_columns(
            "Fecha UTC", "Proyecto", "Acción", "Error"
        )

    def _setup_panel_titles(self) -> None:
        titles = {
            "#summary": "WORKSPACE",
            "#overview-processes": "PROCESOS",
            "#overview-resources": "RECURSOS",
            "#projects-table": "PROYECTOS",
            "#agents-table": "AGENTES",
            "#console-processes": "COMANDOS DEL PROYECTO",
            "#console-output": "TERMINAL · SALIDA EN VIVO",
            "#processes-table": "PROCESOS",
            "#resources-detail": "SISTEMA",
            "#resources-project-summary": "PROYECTO",
            "#resources-processes": "PROCESOS",
            "#resources-all-summary": "GLOBAL",
            "#resources-all-projects": "PROYECTOS",
            "#memory-list": "NOTAS",
            "#memory-detail": "DETALLE",
            "#chat-list": "CHATS",
            "#chat-detail": "CONVERSACIÓN",
            "#errors-table": "ERRORES RECIENTES",
            "#error-detail": "DETALLE DEL ERROR",
            "#brand-banner": "CHXCHX",
        }
        for selector, title in titles.items():
            self.screen.query_one(selector).border_title = title
        self._query("#handoff", Static).border_title = "HANDOFF"

    def _query(self, selector: str, expect_type):
        """Query the active Textual screen, including test/default screens."""
        return self.screen.query_one(selector, expect_type)

    # Textual doesn't register event handlers declared only on plain mixins.
    # These app-level handlers dispatch to the focused, testable event methods.
    @on(Button.Pressed)
    def dispatch_button_pressed(self, event: Button.Pressed) -> None:
        button_id = event.button.id
        if not button_id or not button_id.startswith("btn-"):
            return
        method_name = "button_" + button_id.removeprefix("btn-").replace("-", "_")
        handler = getattr(self, method_name, None)
        if handler is not None:
            handler()
        # Keep application shortcuts usable after a mouse click focuses a button.
        self.set_focus(None)

    @on(Input.Changed)
    def dispatch_input_changed(self, event: Input.Changed) -> None:
        handlers = {
            "chat-search": self.chat_search_changed,
            "memory-search": self.memory_search_changed,
        }
        handler = handlers.get(event.input.id)
        if handler is not None:
            handler(event)

    @on(DataTable.RowHighlighted)
    def dispatch_table_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        handlers = {
            "chat-list": self.chat_row_highlighted,
            "errors-table": self.error_row_highlighted,
            "console-processes": self.console_process_highlighted,
            "memory-list": self.memory_row_highlighted,
            "agents-table": self.agent_row_highlighted,
        }
        handler = handlers.get(event.data_table.id)
        if handler is not None:
            handler(event)

    # Navigation and command palette
