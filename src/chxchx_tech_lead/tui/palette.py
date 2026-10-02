from __future__ import annotations

from textual import on
from textual.app import ComposeResult
from textual.containers import Container
from textual.events import Key
from textual.screen import ModalScreen
from textual.widgets import Input, Label, ListItem, ListView


class PaletteInput(Input):
    def on_key(self, event: Key) -> None:
        if event.key == "escape":
            event.stop()
            self.screen.dismiss(None)


class CommandPalette(ModalScreen[str | None]):
    """Small local command palette, independent of Textual's optional providers."""

    CSS = """
    CommandPalette {
        align: center middle;
        background: #1d1f29 85%;
    }
    #palette-box {
        width: 70;
        height: auto;
        max-height: 80%;
        padding: 1 2;
        background: #2e303d;
        border: solid #aeb1c2;
    }
    #palette-title {
        padding: 0 0 1 0;
        color: #ddd6ff;
        text-style: bold;
    }
    #palette-input {
        background: #303240;
        color: #ececf1;
        border: solid #aeb1c2;
    }
    #palette-input:focus { border: solid #d6caff; }
    #palette-list {
        height: auto;
        max-height: 18;
        margin-top: 1;
        background: #2e303d;
    }
    #palette-list > ListItem { padding: 0 1; color: #ececf1; }
    #palette-list > ListItem.-hovered,
    #palette-list > ListItem.-highlight {
        background: #444758;
        color: #ffffff;
    }
    """
    COMMANDS = (
        ("overview", "Ver resumen", "1"),
        ("projects", "Ver proyectos", "2"),
        ("agents", "Ver agentes", "3"),
        ("processes", "Ver procesos", "4"),
        ("resources", "Ver recursos", "5"),
        ("handoff", "Ver handoff", "6"),
        ("memory", "Ver notas del proyecto", "7"),
        ("conversations", "Ver historial de chats", "8"),
        ("errors", "Ver errores recientes", "F2"),
        ("guide", "Ver guía de uso", "F1"),
        ("setup", "Abrir configuración e init", "0"),
        ("attach_agent_terminal", "Abrir terminal del agente", "g"),
        ("brand", "Ver sello CHXCHX-DEV", "9"),
        ("refresh", "Actualizar panel", "r"),
        ("open_workspace", "Abrir workspace", "o"),
        ("attach_workspace", "Adjuntar a terminales", "j"),
        ("attach_agents_workspace", "Adjuntar a pestaña Agentes", ""),
        ("trust_workspace", "Confiar este proyecto", "y"),
        ("start_project", "Iniciar proyecto", "s"),
        ("start_workspace_all", "Abrir Zellij con agentes", "t"),
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
