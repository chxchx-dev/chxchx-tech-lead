from __future__ import annotations

from textual.app import App, ComposeResult
from textual.containers import Horizontal
from textual.widgets import (
    Button,
    DataTable,
    Footer,
    Header,
    Input,
    Label,
    Static,
    TabbedContent,
    TabPane,
)

from ..core.models import ProjectInfo
from ..integrations.chat_history import Conversation
from ..workspace.agent_status import AgentRuntimeStatus
from ..workspace.memory_history import MemoryNote
from ..workspace.service import WorkspaceService
from .actions import WorkspaceActions
from .branding import BRAND_BANNER
from .dashboard import WorkspaceDashboard
from .events import WorkspaceEvents
from .panels import WorkspacePanels
from .palette import CommandPalette
from .project_console import WorkspaceProjectConsole
from .style import TUI_BINDINGS, TUI_CSS


class WorkspaceConsole(
    WorkspaceActions,
    WorkspaceEvents,
    WorkspaceDashboard,
    WorkspacePanels,
    WorkspaceProjectConsole,
    App[None],
):
    TITLE = "ChxChx Terminal Workspace"
    CSS = TUI_CSS
    BINDINGS = TUI_BINDINGS

    def __init__(self, project: ProjectInfo) -> None:
        super().__init__()
        self.project = project
        self.service = WorkspaceService(project)
        self._last_inspection = None
        self._last_agents: list[AgentRuntimeStatus] = []
        self._memory_notes: dict[str, MemoryNote] = {}
        self._conversations: dict[str, Conversation] = {}
        self._palette_open = False
        self._attach_pending = False
        self._start_pending = False
        self._console_process_id: str | None = None
        self._console_processes = {}

    @property
    def subtitle(self) -> str:
        return f"{self.project.name} · {self.project.profile_name}"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with TabbedContent(initial="overview", id="tabs"):
            with TabPane("Resumen", id="overview"):
                yield Static(
                    "CHXCHX-DEV SYSTEM · BUILD • AUTOMATE • CREATE",
                    id="brand",
                )
                yield Static(
                    "Flujo recomendado: 1) Confiar proyecto  2) Iniciar workspace + agentes  "
                    "3) Reintentar agentes  4) Adjuntar a Zellij.\n"
                    "No necesitas escribir nada para estas acciones; los IDs solo se usan en Agentes y Procesos.\n"
                    "Dentro de Zellij vuelve con Ctrl+O y después D; si la sesión no existe verás un error aquí.",
                    id="action-guide",
                    classes="action-guide",
                )
                with Horizontal(classes="toolbar"):
                    yield Button("Preparar", id="btn-open")
                    yield Button("1. Confiar proyecto", id="btn-trust")
                    yield Button("2. Iniciar proyecto", id="btn-start", variant="success")
                    yield Button("3. Workspace + agentes", id="btn-start-all", variant="primary")
                with Horizontal(classes="toolbar"):
                    yield Button("4. Adjuntar Zellij", id="btn-attach")
                    yield Button("Detener", id="btn-stop", variant="error")
                    yield Button("Actualizar", id="btn-refresh")
                yield Static(id="summary", classes="summary")
                with Horizontal(classes="wide"):
                    yield DataTable(id="overview-processes")
                    yield Static(id="overview-resources", classes="side-panel")
            with TabPane("Consola", id="console"):
                yield Static(id="console-technology", classes="summary")
                yield DataTable(id="console-processes", cursor_type="row")
                with Horizontal(classes="toolbar"):
                    yield Button("Iniciar proyecto", id="btn-console-start", variant="success")
                    yield Button("Detener proyecto", id="btn-console-stop", variant="error")
                    yield Button("Confiar proyecto", id="btn-console-trust")
                    yield Button("Actualizar", id="btn-console-refresh")
                yield Static(
                    "Selecciona un proceso para ver su comando y salida.",
                    id="console-output",
                    markup=False,
                )
            with TabPane("Proyectos", id="projects"):
                yield Static(
                    "Proyectos registrados · escribe un alias o ruta para cambiar el contexto",
                    classes="summary",
                )
                with Horizontal(classes="toolbar"):
                    yield Input(value=str(self.project.root), id="project-ref")
                    yield Button("Cambiar proyecto", id="btn-switch", variant="primary")
                    yield Button("Actualizar", id="btn-projects-refresh")
                yield DataTable(id="projects-table")
            with TabPane("Agentes", id="agents"):
                yield Static(
                    "Selecciona una fila para elegir el agente; el ID se completa automáticamente. Disponibilidad CLI, sesión y pane detectado.",
                    classes="summary",
                )
                yield Static(id="autosave-status", classes="summary")
                with Horizontal(classes="toolbar"):
                    yield Label("ID de agente:", classes="input-label")
                    yield Input(placeholder="codex o claude", id="agent-id", classes="field")
                    yield Button("Iniciar seleccionado", id="btn-agent-start", variant="primary")
                    yield Button("Nuevo chat + contexto", id="btn-agent-new-chat")
                    yield Button("Iniciar todos", id="btn-agents", variant="success")
                    yield Button("Actualizar", id="btn-agents-refresh")
                with Horizontal(classes="toolbar"):
                    yield Button(
                        "Abrir terminal del agente seleccionado",
                        id="btn-agent-attach",
                        variant="primary",
                    )
                yield DataTable(id="agents-table", cursor_type="row")
            with TabPane("Procesos", id="processes"):
                yield Static("Procesos declarados en .ai/chxchx-tech.toml", classes="summary")
                with Horizontal(classes="toolbar"):
                    yield Label("ID de proceso:", classes="input-label")
                    yield Input(placeholder="ej. frontend o api", id="process-id", classes="field")
                    yield Button("Iniciar", id="btn-process-start", variant="success")
                    yield Button("Detener", id="btn-process-stop", variant="error")
                    yield Button("Actualizar", id="btn-process-refresh")
                yield DataTable(id="processes-table")
            with TabPane("Recursos", id="resources"):
                yield Static(id="resources-detail", classes="summary")
                yield Static(id="resources-project-summary", classes="summary")
                yield DataTable(id="resources-processes")
                yield Static(id="resources-all-summary", classes="summary")
                yield DataTable(id="resources-all-projects")
                with Horizontal(classes="toolbar"):
                    yield Button("Actualizar recursos", id="btn-resources-refresh")
                    yield Button("Detener workspace", id="btn-resources-stop", variant="error")
            with TabPane("Handoff", id="handoff"):
                yield Static(id="handoff", classes="wide")
                yield Label("Resumen del último cambio", classes="handoff-label")
                yield Input(value="Sesión administrada desde el TUI", id="handoff-summary")
                yield Label("Pendiente", classes="handoff-label")
                yield Input(value="Continuar el trabajo del proyecto", id="handoff-pending")
                yield Label("Validación", classes="handoff-label")
                yield Input(value="Ejecutar las pruebas del proyecto", id="handoff-validation")
                with Horizontal(classes="toolbar"):
                    yield Button("Actualizar handoff", id="btn-handoff", variant="primary")
                    yield Button("Recargar", id="btn-handoff-refresh")
            with TabPane("Notas", id="memory"):
                yield Static(
                    "Historial de notas persistentes de este proyecto · solo lectura · ordenado por última modificación",
                    id="memory-summary",
                    classes="summary",
                )
                with Horizontal(classes="toolbar"):
                    yield Input(placeholder="Buscar en títulos y notas...", id="memory-search")
                    yield Button("Actualizar", id="btn-memory-refresh")
                with Horizontal(classes="wide"):
                    yield DataTable(id="memory-list", cursor_type="row")
                    yield Static(
                        "Selecciona una nota para ver su contenido.",
                        id="memory-detail",
                        markup=False,
                    )
            with TabPane("Chats", id="conversations"):
                yield Static(
                    "Conversaciones guardadas localmente por proyecto · solo lectura",
                    id="chat-summary",
                    classes="summary",
                )
                with Horizontal(classes="toolbar"):
                    yield Input(placeholder="Buscar en chats...", id="chat-search")
                    yield Button("Actualizar", id="btn-chat-refresh")
                with Horizontal(classes="wide"):
                    yield DataTable(id="chat-list", cursor_type="row")
                    yield Static(
                        "Selecciona una conversación para leerla.",
                        id="chat-detail",
                        markup=False,
                    )
            with TabPane("Marca", id="brand-tab"):
                yield Static(BRAND_BANNER, id="brand-banner", markup=False)
        yield Static("Listo.", id="log")
        yield Footer()

    def on_mount(self) -> None:
        self.sub_title = self.subtitle
        self._setup_tables()
        self._setup_panel_titles()
        self._refresh_project_console()
        self.set_interval(3, self.refresh_dashboard)
        self.set_interval(1, self._refresh_project_output)
        self.refresh_dashboard()
        self._refresh_conversations()

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
            "Agente", "CLI", "Disponible", "Sesión", "Pane", "Preset", "Versión"
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

    def _setup_panel_titles(self) -> None:
        titles = {
            "#summary": "WORKSPACE",
            "#overview-processes": "PROCESOS",
            "#overview-resources": "RECURSOS",
            "#projects-table": "PROYECTOS",
            "#autosave-status": "AUTOGUARDADO",
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
            "#brand-banner": "CHXCHX",
        }
        for selector, title in titles.items():
            self.screen.query_one(selector).border_title = title
        self._query("#handoff", Static).border_title = "HANDOFF"

    def _query(self, selector: str, expect_type):
        """Query the active Textual screen, including test/default screens."""
        return self.screen.query_one(selector, expect_type)

    # Navigation and command palette
