from __future__ import annotations

from textual.app import ComposeResult
from textual.containers import Horizontal
from textual.widgets import Button, DataTable, Footer, Header, Input, Label, Static, TabbedContent, TabPane

from ..core.models import ProjectInfo
from .branding import BRAND_BANNER


def compose_workspace(project: ProjectInfo) -> ComposeResult:
    """Compose four primary areas and keep infrequent tools under Más."""
    yield Header(show_clock=True)
    with TabbedContent(initial="overview", id="tabs"):
        yield from _overview()
        with TabPane("Trabajo", id="work"):
            with TabbedContent(initial="console", id="work-tabs"):
                yield from _console()
                yield from _agents()
                yield from _processes()
        yield from _projects(project)
        with TabPane("Más", id="more"):
            yield Static("Herramientas avanzadas y ayuda", classes="summary")
            with TabbedContent(initial="guide", id="more-tabs"):
                yield from _resources()
                yield from _handoff()
                yield from _memory()
                yield from _conversations()
                yield from _errors()
                yield from _guide()
                yield from _setup()
                yield from _brand()
    yield Static("Listo.", id="log")
    yield Footer()


def _overview() -> ComposeResult:
    with TabPane("Inicio", id="overview"):
        yield Static("CHXCHX-DEV SYSTEM · BUILD • AUTOMATE • CREATE", id="brand")
        yield Static(
            "Empieza aquí: confía el proyecto si hace falta y luego inicia tu sesión de trabajo.",
            classes="summary",
        )
        with Horizontal(classes="toolbar"):
            yield Button("Confiar proyecto", id="btn-trust")
            yield Button("Iniciar y abrir", id="btn-start-all", variant="primary")
            yield Button("Detener procesos", id="btn-stop", variant="error")
            yield Button("Actualizar", id="btn-refresh")
        yield Static(id="summary", classes="summary")
        with Horizontal(classes="wide"):
            yield DataTable(id="overview-processes")
            yield Static(id="overview-resources", classes="side-panel")


def _console() -> ComposeResult:
    with TabPane("Proyecto", id="console"):
        yield Static(id="console-technology", classes="summary")
        yield DataTable(id="console-processes", cursor_type="row")
        with Horizontal(classes="toolbar"):
            yield Button("Iniciar procesos", id="btn-console-start", variant="success")
            yield Button("Detener procesos", id="btn-console-stop", variant="error")
            yield Button("Confiar proyecto", id="btn-console-trust")
            yield Button("Terminales Zellij", id="btn-attach")
            yield Button("Nueva terminal", id="btn-terminal")
            yield Button("Actualizar", id="btn-console-refresh")
        yield Static("Selecciona un proceso para ver su comando y salida.", id="console-output", markup=False)


def _agents() -> ComposeResult:
    with TabPane("Agentes", id="agents"):
        yield Static(
            "Selecciona un agente para abrirlo o iniciar un chat nuevo.",
            classes="summary",
        )
        with Horizontal(classes="toolbar"):
            yield Label("Agente:", classes="input-label")
            yield Input(placeholder="codex o claude", id="agent-id", classes="field")
            yield Button("Iniciar seleccionado", id="btn-agent-start", variant="primary")
            yield Button("Nuevo chat", id="btn-agent-new-chat")
            yield Button("Iniciar todos", id="btn-agents", variant="success")
            yield Button("Actualizar", id="btn-agents-refresh")
        with Horizontal(classes="toolbar"):
            yield Button("Abrir terminal del agente", id="btn-agent-attach", variant="primary")
        yield DataTable(id="agents-table", cursor_type="row")


def _processes() -> ComposeResult:
    with TabPane("Procesos", id="processes"):
        yield Static("Comandos de desarrollo configurados para este proyecto.", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Label("Proceso:", classes="input-label")
            yield Input(placeholder="ej. frontend o api", id="process-id", classes="field")
            yield Button("Iniciar", id="btn-process-start", variant="success")
            yield Button("Detener", id="btn-process-stop", variant="error")
            yield Button("Actualizar", id="btn-process-refresh")
        yield DataTable(id="processes-table")


def _projects(project: ProjectInfo) -> ComposeResult:
    with TabPane("Proyectos", id="projects"):
        yield Static("Cambia el proyecto activo o vuelve a uno registrado.", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Input(value=str(project.root), id="project-ref")
            yield Button("Cambiar proyecto", id="btn-switch", variant="primary")
            yield Button("Actualizar", id="btn-projects-refresh")
        yield DataTable(id="projects-table")


def _resources() -> ComposeResult:
    with TabPane("Recursos", id="resources"):
        yield Static(id="resources-detail", classes="summary")
        yield Static(id="resources-project-summary", classes="summary")
        yield DataTable(id="resources-processes")
        yield Static(id="resources-all-summary", classes="summary")
        yield DataTable(id="resources-all-projects")
        with Horizontal(classes="toolbar"):
            yield Button("Actualizar recursos", id="btn-resources-refresh")
            yield Button("Detener workspace", id="btn-resources-stop", variant="error")


def _handoff() -> ComposeResult:
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


def _memory() -> ComposeResult:
    with TabPane("Notas", id="memory"):
        yield Static("Notas persistentes del proyecto · solo lectura", id="memory-summary", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Input(placeholder="Buscar en títulos y notas...", id="memory-search")
            yield Button("Actualizar", id="btn-memory-refresh")
        with Horizontal(classes="wide"):
            yield DataTable(id="memory-list", cursor_type="row")
            yield Static("Selecciona una nota para ver su contenido.", id="memory-detail", markup=False)


def _conversations() -> ComposeResult:
    with TabPane("Chats", id="conversations"):
        yield Static("Conversaciones guardadas localmente · solo lectura", id="chat-summary", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Input(placeholder="Buscar en chats...", id="chat-search")
            yield Button("Actualizar", id="btn-chat-refresh")
        with Horizontal(classes="wide"):
            yield DataTable(id="chat-list", cursor_type="row")
            yield Static("Selecciona una conversación para leerla.", id="chat-detail", markup=False)


def _errors() -> ComposeResult:
    with TabPane("Errores", id="errors"):
        yield Static("Errores recientes de la TUI guardados localmente", id="errors-summary", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Button("Actualizar errores", id="btn-errors-refresh")
        with Horizontal(classes="wide"):
            yield DataTable(id="errors-table", cursor_type="row")
            yield Static("Selecciona un error para ver el detalle.", id="error-detail", markup=False)


def _guide() -> ComposeResult:
    with TabPane("Guía", id="guide"):
        yield Static(
            "FLUJO RÁPIDO\n"
            "1. Confía el proyecto si ChxChx lo solicita.\n"
            "2. Pulsa «Iniciar y abrir» para iniciar procesos, preparar agentes y entrar a Zellij.\n"
            "3. Para volver al TUI, pulsa Ctrl+O y después D dentro de Zellij.\n\n"
            "TRABAJO\n"
            "La pestaña Trabajo reúne comandos del proyecto, agentes y terminales.\n"
            "Detener procesos cierra los procesos administrados y conserva los archivos.\n\n"
            "AYUDA\n"
            "Pulsa Ctrl+P para buscar acciones. «Más» contiene configuración, recursos, notas e historial.",
            id="usage-guide",
            markup=False,
            classes="usage-guide",
        )


def _setup() -> ComposeResult:
    with TabPane("Configuración", id="setup"):
        yield Static("Inicializa este repositorio, instala herramientas y configura integraciones.", classes="summary")
        with Horizontal(classes="toolbar"):
            yield Button("Previsualizar init", id="btn-init-preview")
            yield Button("Inicializar proyecto", id="btn-init", variant="primary")
            yield Button("Init mínimo", id="btn-init-minimal")
        with Horizontal(classes="toolbar"):
            yield Button("Instalar herramientas", id="btn-install-tools")
            yield Button("Configurar MCP", id="btn-integrate")
            yield Button("Preparación completa", id="btn-setup-all", variant="success")
            yield Button("Diagnóstico", id="btn-doctor")
        yield Static(
            "La vista previa no escribe archivos. Inicializar conserva un backup de los archivos administrados.",
            id="setup-output",
            markup=False,
            classes="summary",
        )


def _brand() -> ComposeResult:
    with TabPane("Marca", id="brand-tab"):
        yield Static(BRAND_BANNER, id="brand-banner", markup=False)
