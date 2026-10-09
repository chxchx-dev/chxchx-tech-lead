from __future__ import annotations

from textual import on
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.widgets import (
    Button,
    DataTable,
    Input,
    TabbedContent,
)

from ..core.models import ProjectInfo
from ..workspace.service import WorkspaceService
from .agent_actions import WorkspaceAgentActions
from .actions import WorkspaceActions
from .dashboard import WorkspaceDashboard
from .events import WorkspaceEvents
from .event_dispatch import (
    dispatch_button_pressed as route_button_pressed,
    dispatch_input_changed as route_input_changed,
    dispatch_table_row_highlighted as route_table_row_highlighted,
)
from .governor_actions import ResourceGovernorActions
from .layout import compose_workspace
from .lifecycle import activate_tab, mount_workspace, unmount_workspace
from .panels import WorkspacePanels
from .palette import CommandPalette
from .project_console import WorkspaceProjectConsole
from .style import TUI_BINDINGS, TUI_CSS
from .skills_actions import SkillsActions
from .skills_panel import SkillsPanel
from .session_state import SessionField, WorkspaceSessionState
from .setup_actions import WorkspaceSetupActions
from .terminal_actions import WorkspaceTerminalActions


class WorkspaceConsole(
    ResourceGovernorActions,
    WorkspaceAgentActions,
    WorkspaceActions,
    WorkspaceSetupActions,
    WorkspaceTerminalActions,
    WorkspaceEvents,
    WorkspaceDashboard,
    WorkspacePanels,
    WorkspaceProjectConsole,
    SkillsActions,
    SkillsPanel,
    App[None],
):
    _last_inspection = SessionField()
    _dashboard_pending = SessionField()
    _dashboard_refresh_again = SessionField()
    _last_agents = SessionField()
    _memory_notes = SessionField()
    _skill_recommendations = SessionField()
    _skill_active = SessionField()
    _skill_registry = SessionField()
    _tech_pack_registry = SessionField()
    _pack_matches = SessionField()
    _conversations = SessionField()
    _cached_errors = SessionField()
    _palette_open = SessionField()
    _attach_pending = SessionField()
    _start_pending = SessionField()
    _operation_pending = SessionField()
    _setup_pending = SessionField()
    _governor_pending = SessionField()
    _console_process_id = SessionField()
    _console_processes = SessionField()
    _console_pending = SessionField()
    _console_refresh_again = SessionField()
    _console_output_pending = SessionField()
    _conversation_refresh_generation = SessionField()
    _memory_refresh_generation = SessionField()
    _chat_search_timer = SessionField()
    _memory_search_timer = SessionField()
    _tab_refresh_timer = SessionField()

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
        self._session_state = WorkspaceSessionState()

    @property
    def subtitle(self) -> str:
        return f"{self.project.name} · {self.project.profile_name}"

    def compose(self) -> ComposeResult:
        yield from compose_workspace(self.project)

    def on_mount(self) -> None:
        mount_workspace(self)

    def on_unmount(self) -> None:
        unmount_workspace(self)

    @on(TabbedContent.TabActivated)
    def on_tab_activated(self, event: TabbedContent.TabActivated) -> None:
        activate_tab(self, event)

    def _query(self, selector: str, expect_type):
        """Query the active Textual screen, including test/default screens."""
        return self.screen.query_one(selector, expect_type)

    # Textual doesn't register event handlers declared only on plain mixins.
    # These app-level handlers dispatch to the focused, testable event methods.
    @on(Button.Pressed)
    def dispatch_button_pressed(self, event: Button.Pressed) -> None:
        route_button_pressed(self, event)

    @on(Input.Changed)
    def dispatch_input_changed(self, event: Input.Changed) -> None:
        route_input_changed(self, event)

    @on(DataTable.RowHighlighted)
    def dispatch_table_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        route_table_row_highlighted(self, event)

    # Navigation and command palette
