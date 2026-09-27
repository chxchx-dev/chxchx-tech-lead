from __future__ import annotations

from pathlib import Path
from typing import Callable

from ..core.models import ProjectInfo


class TUIUnavailableError(RuntimeError):
    """Textual no está instalado en el entorno actual."""


BRAND_BANNER = r"""╭──────────────────────────────────────────────────────────────╮
│                                                              │
│   ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗          │
│  ██╔════╝██║  ██║╚██╗██╔╝██╔════╝██║  ██║╚██╗██╔╝          │
│  ██║     ███████║ ╚███╔╝ ██║     ███████║ ╚███╔╝           │
│  ██║     ██╔══██║ ██╔██╗ ██║     ██╔══██║ ██╔██╗           │
│  ╚██████╗██║  ██║██╔╝ ██╗╚██████╗██║  ██║██╔╝ ██╗          │
│   ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝          │
│                                                              │
│                       - D E V -                              │
│                                                              │
│                   CHXCHX-DEV SYSTEM                          │
│                                                              │
│              [ BUILD • AUTOMATE • CREATE ]                   │
│                                                              │
╰──────────────────────────────────────────────────────────────╯"""


def run_tui(project: ProjectInfo) -> None:
    """Start the interactive workspace console."""
    try:
        from textual import on
        from textual.app import App, ComposeResult
        from textual.containers import Container, Horizontal
        from textual.events import Key
        from textual.screen import ModalScreen
        from textual.widgets import (
            Button,
            DataTable,
            Footer,
            Header,
            Input,
            Label,
            ListItem,
            ListView,
            Static,
            TabbedContent,
            TabPane,
        )
    except ImportError as exc:
        raise TUIUnavailableError(
            "Textual no está instalado; instala la dependencia de TUI antes de ejecutar `chxchx-tech tui`."
        ) from exc

    from ..core.detector import detect_project
    from ..core.registry import load_registry, resolve_project_reference
    from ..core.trust import trust_project
    from ..workspace.agent_status import AgentRuntimeStatus
    from ..workspace.autosave_status import diagnose_autosave, format_autosave_status
    from ..workspace.handoff import update_handoff
    from ..workspace.memory_history import MemoryNote, list_memory_notes
    from ..workspace.process_manager import ProcessManager
    from ..workspace.resources import (
        ResourceManager,
        format_bytes,
        summarize_process_resources,
    )
    from ..workspace.service import WorkspaceOperationError, WorkspaceService
    from ..workspace.state import load_state

    class PaletteInput(Input):
        def on_key(self, event: Key) -> None:
            if event.key == "escape":
                event.stop()
                self.screen.dismiss(None)

    class CommandPalette(ModalScreen[str | None]):
        """Small local command palette, independent of Textual's optional providers."""

        CSS = """
        CommandPalette { align: center middle; }
        #palette-box {
            width: 70;
            height: auto;
            max-height: 80%;
            padding: 1 2;
            background: $panel;
            border: round $primary;
        }
        #palette-title { padding: 0 0 1 0; color: $text-muted; }
        #palette-list { height: auto; max-height: 18; }
        """
        COMMANDS = (
            ("overview", "Ver resumen", "1"),
            ("projects", "Ver proyectos", "2"),
            ("agents", "Ver agentes", "3"),
            ("processes", "Ver procesos", "4"),
            ("resources", "Ver recursos", "5"),
            ("handoff", "Ver handoff", "6"),
            ("memory", "Ver memoria e historial", "7"),
            ("brand", "Ver sello CHXCHX-DEV", "8"),
            ("refresh", "Actualizar panel", "r"),
            ("open_workspace", "Abrir workspace", "o"),
            ("attach_workspace", "Adjuntar a Zellij", "j"),
            ("trust_workspace", "Confiar este proyecto", "y"),
            ("start_workspace", "Iniciar workspace", "s"),
            ("start_workspace_all", "Iniciar workspace y agentes", "t"),
            ("start_agents", "Iniciar agentes", "a"),
            ("start_agent_selected", "Iniciar agente seleccionado", "c"),
            ("start_process", "Iniciar proceso seleccionado", "i"),
            ("stop_process", "Detener proceso seleccionado", "k"),
            ("suspend_workspace", "Suspender workspace", "u"),
            ("resume_workspace", "Reanudar workspace", "v"),
            ("stop_workspace", "Detener workspace", "x"),
            ("write_handoff", "Actualizar handoff", "h"),
            ("open_editor", "Abrir Sublime Text", "e"),
        )
        BINDINGS = [("escape", "close", "Cerrar")]

        def compose(self) -> ComposeResult:
            with Container(id="palette-box"):
                yield Label("Paleta de comandos · escribe para filtrar", id="palette-title")
                yield PaletteInput(placeholder="Buscar comando...", id="palette-input")
                yield ListView(id="palette-list")

        def on_mount(self) -> None:
            self._render_commands("")
            self.query_one("#palette-input", Input).focus()

        @on(Input.Changed, "#palette-input")
        def filter_commands(self, event: Input.Changed) -> None:
            self._render_commands(event.value)

        @on(Input.Submitted, "#palette-input")
        def submit_command(self, _event: Input.Submitted) -> None:
            items = self.query_one("#palette-list", ListView).children
            if items:
                self._choose(items[0].id.removeprefix("command-"))

        @on(ListView.Selected, "#palette-list")
        def select_command(self, event: ListView.Selected) -> None:
            if event.item.id:
                self._choose(event.item.id.removeprefix("command-"))

        def action_close(self) -> None:
            self.dismiss(None)

        def _render_commands(self, query: str) -> None:
            list_view = self.query_one("#palette-list", ListView)
            normalized = query.strip().lower()
            list_view.clear()
            for command_id, label, key in self.COMMANDS:
                if normalized and normalized not in f"{label} {command_id}".lower():
                    continue
                list_view.mount(
                    ListItem(Label(f"{label:<28} [{key}]"), id=f"command-{command_id}")
                )

        def _choose(self, command_id: str) -> None:
            self.dismiss(command_id)

    class WorkspaceConsole(App[None]):
        TITLE = "ChxChx Terminal Workspace"
        CSS = """
        Screen { background: $surface; }
        #tabs { height: 1fr; }
        TabPane { padding: 1 2; }
        .summary {
            height: auto;
            min-height: 5;
            padding: 1 2;
            border: round $primary;
        }
        .action-guide {
            height: auto;
            padding: 0 1;
            color: $text-muted;
        }
        .input-label {
            width: auto;
            padding: 1 1 0 0;
            color: $text-muted;
        }
        .handoff-label {
            height: 1;
            padding: 0 1;
            color: $text-muted;
        }
        #brand {
            height: 1;
            padding: 0 1;
            color: $primary;
        }
        #brand-banner {
            height: auto;
            padding: 1 2;
            color: $primary;
        }
        #memory-list { width: 48%; min-width: 30; }
        #memory-detail {
            width: 1fr;
            height: 1fr;
            margin-left: 1;
            padding: 1 2;
            border: round $secondary;
            overflow-y: auto;
        }
        #memory-summary { height: auto; min-height: 2; }
        .toolbar { height: auto; margin: 1 0; }
        .toolbar Button { margin-right: 1; }
        .wide { width: 1fr; height: 1fr; }
        .side-panel {
            width: 30;
            height: 1fr;
            margin-left: 1;
            padding: 1 2;
            border: round $secondary;
        }
        DataTable { height: 1fr; min-height: 8; }
        #log { height: 3; padding: 1 2; border: round $accent; }
        #project-ref { width: 1fr; }
        #handoff {
            height: 1fr;
            padding: 1 2;
            border: round $secondary;
        }
        .field { width: 1fr; }
        """
        BINDINGS = [
            ("q", "quit", "Salir"),
            ("ctrl+p", "command_palette", "Comandos"),
            ("r", "refresh", "Actualizar"),
            ("1", "show_overview", "Resumen"),
            ("2", "show_projects", "Proyectos"),
            ("3", "show_agents", "Agentes"),
            ("4", "show_processes", "Procesos"),
            ("5", "show_resources", "Recursos"),
            ("6", "show_handoff", "Handoff"),
            ("7", "show_memory", "Memoria"),
            ("8", "show_brand", "Marca"),
            ("o", "open_workspace", "Abrir"),
            ("j", "attach_workspace", "Zellij"),
            ("y", "trust_workspace", "Confiar"),
            ("s", "start_workspace", "Iniciar"),
            ("t", "start_workspace_all", "Iniciar todo"),
            ("a", "start_agents", "Agentes"),
            ("c", "start_agent_selected", "Agente"),
            ("i", "start_process", "Proceso"),
            ("k", "stop_process", "Detener proceso"),
            ("u", "suspend_workspace", "Suspender"),
            ("v", "resume_workspace", "Reanudar"),
            ("x", "stop_workspace", "Detener"),
            ("h", "write_handoff", "Guardar handoff"),
            ("e", "open_editor", "Sublime"),
        ]

        def __init__(self) -> None:
            super().__init__()
            self.project = project
            self.service = WorkspaceService(project)
            self._last_inspection = None
            self._last_agents: list[AgentRuntimeStatus] = []
            self._memory_notes: dict[str, MemoryNote] = {}
            self._palette_open = False
            self._attach_pending = False
            self._start_pending = False

        @property
        def subtitle(self) -> str:
            return f"{self.project.name} · {self.project.profile_name}"

        def compose(self) -> ComposeResult:
            yield Header(show_clock=True)
            yield Static("Listo.", id="log")
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
                        yield Button("2. Iniciar workspace", id="btn-start", variant="success")
                        yield Button("3. Reintentar agentes", id="btn-start-all", variant="primary")
                    with Horizontal(classes="toolbar"):
                        yield Button("4. Adjuntar Zellij", id="btn-attach")
                        yield Button("Detener", id="btn-stop", variant="error")
                        yield Button("Actualizar", id="btn-refresh")
                    yield Static(id="summary", classes="summary")
                    with Horizontal(classes="wide"):
                        yield DataTable(id="overview-processes")
                        yield Static(id="overview-resources", classes="side-panel")
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
                with TabPane("Memoria", id="memory"):
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
                with TabPane("Marca", id="brand-tab"):
                    yield Static(BRAND_BANNER, id="brand-banner", markup=False)
            yield Footer()

        def on_mount(self) -> None:
            self.sub_title = self.subtitle
            self._setup_tables()
            self.set_interval(3, self.refresh_dashboard)
            self.refresh_dashboard()

        def _setup_tables(self) -> None:
            self._query("#overview-processes", DataTable).add_columns(
                "ID", "Estado", "PID", "Puerto", "RAM", "CPU"
            )
            self._query("#processes-table", DataTable).add_columns(
                "ID", "Estado", "PID", "Puerto", "RAM", "CPU"
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

        def _query(self, selector: str, expect_type):
            """Query the active Textual screen, including test/default screens."""
            return self.screen.query_one(selector, expect_type)

        # Navigation and command palette
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
            self.action_start_workspace()

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
            self._set_log(f"Agente seleccionado: {agent_id} · puedes iniciar uno nuevo con contexto")

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
        def refresh_dashboard(self) -> None:
            # The command palette is a modal screen. Its widgets replace the
            # main screen while it is open, so wait for the dismiss callback.
            if self._palette_open:
                return
            try:
                inspection = self.service.inspect()
                self._last_inspection = inspection
                summary = self._query("#summary", Static)
                summary.update(
                    f"Proyecto: {self.project.name}  |  Perfil: {self.project.profile_name}\n"
                    f"Workspace: {inspection.state.status.value}  |  Trust: {'sí' if inspection.trusted else 'no'}\n"
                    f"Stack: {', '.join(self.project.stacks) or 'sin detectar'}\n"
                    f"Ruta: {self.project.root}"
                )
                if self._query("#tabs", TabbedContent).active == "resources":
                    self._refresh_aggregated_resources()
                if inspection.config is None:
                    self._set_log("No hay configuración válida de workspace")
                    return
                resources = ResourceManager(inspection.config.resources)
                system = resources.system()
                self._update_resources(resources, system)
                records = ProcessManager(
                    self.project.root,
                    inspection.config.processes,
                    trusted=inspection.trusted,
                ).list()
                process_metrics = resources.processes(records)
                self._update_project_resources(process_metrics, system.total_bytes)
                self._refresh_process_table(records, process_metrics, "#overview-processes")
                self._refresh_process_table(records, process_metrics, "#processes-table")
                self._refresh_agents()
                self._refresh_projects()
                self._refresh_handoff()
            except (WorkspaceOperationError, OSError) as exc:
                self._set_log(f"Error: {exc}")

        def _update_resources(self, resources, system) -> None:
            cpu = "N/D" if system.cpu_percent is None else f"{system.cpu_percent:.0f}%"
            text = (
                f"RAM   {format_bytes(system.used_bytes)} / {format_bytes(system.total_bytes)} "
                f"({system.memory_percent:.0f}%)\n"
                f"Swap  {format_bytes(system.swap_used_bytes)} / {format_bytes(system.swap_total_bytes)} "
                f"({system.swap_percent:.0f}%)\n"
                f"CPU   {cpu}\n"
                f"Nivel {resources.severity(system).value}"
            )
            self._query("#overview-resources", Static).update(text)
            self._query("#resources-detail", Static).update(
                f"Uso general del equipo (no exclusivo de este proyecto)\n"
                f"Proyecto: {self.project.name}\n\n{text}\n\n"
                f"Umbral RAM aviso: {resources.config.warn_memory_percent}%\n"
                f"Umbral RAM crítico: {resources.config.critical_memory_percent}%\n"
                f"Umbral swap aviso: {resources.config.warn_swap_percent}%"
            )

        def _update_project_resources(self, metrics, total_memory_bytes: int) -> None:
            summary = summarize_process_resources(metrics)
            memory_percent = summary.memory_percent(total_memory_bytes)
            memory_share = (
                "N/D" if memory_percent is None else f"{memory_percent:.1f}% de la RAM del equipo"
            )
            cpu = (
                "N/D"
                if summary.cpu_percent is None
                else f"{summary.cpu_percent:.1f}% ({summary.measured_cpu_count} medido(s); puede superar 100% en varios núcleos)"
            )
            self._query("#resources-project-summary", Static).update(
                f"Consumo de procesos administrados de {self.project.name}\n"
                f"Activos: {summary.running_count}  |  RAM RSS: {format_bytes(summary.rss_bytes)} ({memory_share})  |  CPU: {cpu}\n"
                "Estimación de PID principales configurados; no incluye procesos hijos ni agentes dentro de Zellij."
            )
            table = self._query("#resources-processes", DataTable)
            table.clear()
            for metric in metrics:
                cpu_value = "N/D" if metric.cpu_percent is None else f"{metric.cpu_percent:.1f}%"
                table.add_row(
                    metric.label,
                    metric.status.value,
                    str(metric.pid or "-"),
                    format_bytes(metric.rss_bytes),
                    cpu_value,
                )

        def _refresh_aggregated_resources(self) -> None:
            table = self._query("#resources-all-projects", DataTable)
            table.clear()
            registry = load_registry()
            entries = list(registry.get("projects", []))
            current_root = self.project.root.resolve()
            known_paths = {
                str(Path(str(item.get("path", ""))).expanduser().resolve())
                for item in entries
                if item.get("path")
            }
            if str(current_root) not in known_paths:
                entries.append(
                    {
                        "alias": self.project.name,
                        "name": self.project.name,
                        "path": str(current_root),
                    }
                )

            rows: list[tuple[str, str, str, str, str, int, float | None]] = []
            for item in entries:
                raw_path = item.get("path")
                if not isinstance(raw_path, str) or not raw_path.strip():
                    continue
                root = Path(raw_path).expanduser()
                alias = str(item.get("alias", root.name))
                name = str(item.get("name", root.name))
                if not root.is_dir():
                    rows.append(("*" if root.resolve() == current_root else "", alias, name, "NO EXISTE", "-", 0, None))
                    continue

                try:
                    root = root.resolve()
                    project = ProjectInfo(root=root, name=name)
                    inspection = WorkspaceService(project).inspect()
                    if inspection.config is None:
                        rows.append(("*" if root == current_root else "", alias, name, "ERROR CONFIG", "-", 0, None))
                        continue
                    managed = ProcessManager(
                        root,
                        inspection.config.processes,
                        trusted=inspection.trusted,
                    ).list(persist=False)
                    manager = ResourceManager(inspection.config.resources)
                    usage = summarize_process_resources(manager.processes(managed))
                    rows.append(
                        (
                            "*" if root == current_root else "",
                            alias,
                            name,
                            inspection.state.status.value,
                            str(usage.running_count),
                            usage.rss_bytes,
                            usage.cpu_percent,
                        )
                    )
                except (WorkspaceOperationError, OSError, ValueError):
                    rows.append(("*" if root.resolve() == current_root else "", alias, name, "ERROR", "-", 0, None))

            rows.sort(key=lambda row: (row[0] != "*", row[1].casefold()))
            for active, alias, name, status, running, rss_bytes, cpu_percent in rows:
                cpu = "N/D" if cpu_percent is None else f"{cpu_percent:.1f}%"
                table.add_row(
                    active,
                    alias,
                    name,
                    status,
                    running,
                    format_bytes(rss_bytes),
                    cpu,
                )

            valid_rows = [row for row in rows if row[3] not in {"ERROR", "ERROR CONFIG", "NO EXISTE"}]
            total_processes = sum(int(row[4]) for row in valid_rows)
            total_rss = sum(row[5] for row in valid_rows)
            cpu_values = [row[6] for row in valid_rows if row[6] is not None]
            total_cpu = "N/D" if not cpu_values else f"{sum(cpu_values):.1f}%"
            self._query("#resources-all-summary", Static).update(
                f"Todos los proyectos registrados: {len(rows)}  |  Procesos activos: {total_processes}  |  "
                f"Suma RSS aprox.: {format_bytes(total_rss)}  |  CPU: {total_cpu}\n"
                "La suma puede diferir del uso físico por memoria compartida; son PID principales administrados."
            )

        def _refresh_process_table(self, records, metrics, selector: str) -> None:
            table = self._query(selector, DataTable)
            table.clear()
            ports = {item.id: item.port for item in records}
            for metric in metrics:
                cpu = "N/D" if metric.cpu_percent is None else f"{metric.cpu_percent:.1f}%"
                table.add_row(
                    metric.process_id,
                    metric.status.value,
                    str(metric.pid or "-"),
                    str(ports.get(metric.process_id) or "-"),
                    format_bytes(metric.rss_bytes),
                    cpu,
                )

        def _refresh_projects(self) -> None:
            table = self._query("#projects-table", DataTable)
            table.clear()
            registry = load_registry()
            last = registry.get("last_project")
            for item in registry.get("projects", []):
                path = Path(str(item.get("path", "")))
                if not path.is_dir():
                    continue
                from ..workspace.state import load_state

                state = load_state(path)
                table.add_row(
                    "✓" if str(path.resolve()) == str(last) else "",
                    str(item.get("alias", "-")),
                    str(item.get("name", path.name)),
                    state.status.value,
                    str(item.get("profile", "-")),
                    str(path),
                )

        def _refresh_agents(self) -> None:
            try:
                autosave = diagnose_autosave(self.project)
                self._query("#autosave-status", Static).update(format_autosave_status(autosave))
            except (OSError, ValueError) as exc:
                self._query("#autosave-status", Static).update(
                    f"Checkpoint automático · no se pudo verificar: {exc}"
                )
            try:
                statuses = self.service.agent_statuses()
            except (WorkspaceOperationError, OSError) as exc:
                self._set_log(f"Agentes: {exc}")
                return
            self._last_agents = statuses
            table = self._query("#agents-table", DataTable)
            table.clear()
            for agent in statuses:
                table.add_row(
                    agent.id,
                    agent.command,
                    "✓" if agent.available else "✗",
                    agent.session,
                    agent.pane,
                    agent.preset,
                    agent.version or "-",
                    key=agent.id,
                )

        def _refresh_handoff(self) -> None:
            path = self.project.root / ".ai" / "HANDOFF.md"
            if not path.exists():
                self._query("#handoff", Static).update(
                    f"No existe todavía:\n{path}\n\nPulsa `h` o el botón para generarlo."
                )
                return
            try:
                content = path.read_text(encoding="utf-8")
            except OSError as exc:
                self._query("#handoff", Static).update(f"No se pudo leer handoff: {exc}")
                return
            self._query("#handoff", Static).update(content)

        def _refresh_memory_history(self) -> None:
            query = self._query("#memory-search", Input).value
            notes = list_memory_notes(self.project.root, query=query)
            table = self._query("#memory-list", DataTable)
            table.clear()
            self._memory_notes = {}
            for note in notes:
                key = str(note.path)
                self._memory_notes[key] = note
                table.add_row(
                    note.title,
                    note.modified_at.strftime("%Y-%m-%d %H:%M"),
                    note.preview,
                    key=key,
                )

            memory_path = self.project.root / ".ai" / "memory"
            if not memory_path.is_dir():
                message = f"Este proyecto todavía no tiene memoria local:\n{memory_path}"
            elif not notes:
                message = "No hay notas que coincidan con la búsqueda."
            else:
                message = f"{len(notes)} nota(s) de {self.project.name} · {memory_path}"
            self._query("#memory-summary", Static).update(message)
            self._query("#memory-detail", Static).update(
                "Selecciona una nota para ver su contenido."
            )

        def _set_log(self, message: str) -> None:
            self._query("#log", Static).update(message)

    WorkspaceConsole().run()
