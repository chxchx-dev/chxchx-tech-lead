from __future__ import annotations

from textual.app import ComposeResult
from textual.containers import Horizontal
from textual.widgets import Button, DataTable, Footer, Header, Input, Label, Static, TabbedContent, TabPane

from ..core.models import ProjectInfo
from .branding import BRAND_BANNER


def compose_workspace(project: ProjectInfo) -> ComposeResult:
    """Compose the tabbed workspace interface for a project."""
    yield Header(show_clock=True)
    with TabbedContent(initial="overview", id="tabs"):
        with TabPane("Resumen", id="overview"):
            yield Static("CHXCHX-DEV SYSTEM · BUILD • AUTOMATE • CREATE", id="brand")
            with Horizontal(classes="toolbar"):
                yield Button("Preparar sesión", id="btn-open")
                yield Button("Confiar proyecto", id="btn-trust")
                yield Button("Iniciar procesos", id="btn-start", variant="success")
                yield Button("Abrir Zellij + agentes", id="btn-start-all", variant="primary")
            with Horizontal(classes="toolbar"):
                yield Button("Adjuntar terminales", id="btn-attach")
                yield Button("Adjuntar agentes", id="btn-attach-agents", variant="primary")
                yield Button("Nueva terminal", id="btn-terminal")
                yield Button("Detener procesos", id="btn-stop", variant="error")
                yield Button("Actualizar", id="btn-refresh")
            yield Static(id="summary", classes="summary")
            with Horizontal(classes="wide"):
                yield DataTable(id="overview-processes")
                yield Static(id="overview-resources", classes="side-panel")
        with TabPane("Guía de uso", id="guide"):
            yield Static("Guía rápida · sigue los pasos según lo que quieras hacer", classes="summary")
            yield Static(
                "INICIAR EL WORKSPACE\n"
                "1. Si el proyecto no es confiable, pulsa «Confiar proyecto».\n"
                "2. «Preparar sesión» crea la sesión y su layout de Zellij.\n"
                "3. «Abrir Zellij + agentes» inicia el workspace, sus procesos de autoarranque y los agentes configurados (por ejemplo, Codex y Claude), y entra a Zellij.\n"
                "4. Para volver al TUI, dentro de Zellij pulsa Ctrl+O, suelta las teclas y luego pulsa D.\n\n"
                "Zellij separa el trabajo en dos pestañas: «Agentes» contiene los agentes configurados y el panel de uso/tokens; «Terminales» contiene las shells normales. Usa «Adjuntar agentes» o «Adjuntar terminales» para entrar directamente a cada espacio.\n\n"
                "ABRIR OTRA TERMINAL\n"
                "Pulsa «Nueva terminal» en Resumen. Se crea una shell en la carpeta del proyecto y se abre Zellij.\n"
                "Para regresar aquí, dentro de Zellij pulsa Ctrl+O, suelta las teclas y luego pulsa D.\n"
                "La sesión Zellij debe estar activa. Si aparece como EXITED, vuelve a pulsar «Abrir Zellij + agentes» y revisa Errores.\n\n"
                "PROCESOS DEL PROYECTO\n"
                "«Iniciar procesos» ejecuta los comandos definidos en .ai/chxchx-tech.toml; no inicia agentes.\n"
                "La pestaña Consola muestra sus comandos y salida. «Detener procesos» detiene los procesos administrados.\n\n"
                "DÓNDE ENCONTRAR AYUDA\n"
                "La pestaña Configuración (tecla 0) permite previsualizar o ejecutar init, instalar herramientas, configurar MCP y correr el diagnóstico.\n"
                "La pestaña Errores muestra los errores guardados para este proyecto. Pulsa F2 para abrirla.\n"
                "La paleta de comandos se abre con Ctrl+P. Los atajos disponibles aparecen en la barra inferior.",
                id="usage-guide",
                markup=False,
                classes="usage-guide",
            )
        with TabPane("Configuración", id="setup"):
            yield Static(
                "Inicializa este repositorio, instala las herramientas y configura las integraciones sin salir de la TUI.",
                classes="summary",
            )
            with Horizontal(classes="toolbar"):
                yield Button("Previsualizar init", id="btn-init-preview")
                yield Button("Inicializar proyecto", id="btn-init", variant="primary")
                yield Button("Init mínimo", id="btn-init-minimal")
            with Horizontal(classes="toolbar"):
                yield Button("Instalar herramientas base", id="btn-install-tools")
                yield Button("Configurar MCP", id="btn-integrate")
                yield Button("Preparación completa", id="btn-setup-all", variant="success")
                yield Button("Diagnóstico", id="btn-doctor")
            yield Static(
                "La vista previa no escribe archivos. Inicializar crea un backup antes de actualizar archivos administrados.",
                id="setup-output",
                markup=False,
                classes="summary",
            )
        with TabPane("Consola", id="console"):
            yield Static(id="console-technology", classes="summary")
            yield DataTable(id="console-processes", cursor_type="row")
            with Horizontal(classes="toolbar"):
                yield Button("Iniciar proyecto", id="btn-console-start", variant="success")
                yield Button("Detener proyecto", id="btn-console-stop", variant="error")
                yield Button("Confiar proyecto", id="btn-console-trust")
                yield Button("Actualizar", id="btn-console-refresh")
            yield Static("Selecciona un proceso para ver su comando y salida.", id="console-output", markup=False)
        with TabPane("Proyectos", id="projects"):
            yield Static("Proyectos registrados · escribe un alias o ruta para cambiar el contexto", classes="summary")
            with Horizontal(classes="toolbar"):
                yield Input(value=str(project.root), id="project-ref")
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
                yield Button("Abrir terminal del agente seleccionado", id="btn-agent-attach", variant="primary")
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
                yield Static("Selecciona una nota para ver su contenido.", id="memory-detail", markup=False)
        with TabPane("Chats", id="conversations"):
            yield Static("Conversaciones guardadas localmente por proyecto · solo lectura", id="chat-summary", classes="summary")
            with Horizontal(classes="toolbar"):
                yield Input(placeholder="Buscar en chats...", id="chat-search")
                yield Button("Actualizar", id="btn-chat-refresh")
            with Horizontal(classes="wide"):
                yield DataTable(id="chat-list", cursor_type="row")
                yield Static("Selecciona una conversación para leerla.", id="chat-detail", markup=False)
        with TabPane("Errores", id="errors"):
            yield Static(
                "Errores recientes de la TUI · se guardan localmente en la caché de ChxChx",
                id="errors-summary",
                classes="summary",
            )
            with Horizontal(classes="toolbar"):
                yield Button("Actualizar errores", id="btn-errors-refresh")
            with Horizontal(classes="wide"):
                yield DataTable(id="errors-table", cursor_type="row")
                yield Static("Selecciona un error para ver el detalle.", id="error-detail", markup=False)
        with TabPane("Marca", id="brand-tab"):
            yield Static(BRAND_BANNER, id="brand-banner", markup=False)
    yield Static("Listo.", id="log")
    yield Footer()
