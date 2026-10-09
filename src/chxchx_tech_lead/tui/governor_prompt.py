from __future__ import annotations

from textual import on
from textual.app import ComposeResult
from textual.containers import Container, Horizontal
from textual.screen import ModalScreen
from textual.widgets import Button, Static


class GovernorPrompt(ModalScreen[bool]):
    """Ask before starting agents above the configured resource budget."""

    CSS = """
    GovernorPrompt { align: center middle; background: #1d1f29 85%; }
    #governor-dialog {
        width: 72; height: auto; max-height: 80%; padding: 1 2;
        background: #2e303d; border: solid #d6bd82;
    }
    #governor-title { color: #f1d99b; text-style: bold; padding-bottom: 1; }
    #governor-message { height: auto; max-height: 12; padding-bottom: 1; }
    #governor-actions { height: auto; align-horizontal: right; }
    #governor-actions Button { width: auto; margin-left: 1; }
    """

    BINDINGS = [("escape", "cancel", "Cancelar")]

    def __init__(self, message: str) -> None:
        super().__init__()
        self.message = message

    def compose(self) -> ComposeResult:
        with Container(id="governor-dialog"):
            yield Static("Aviso de RAM Governor", id="governor-title")
            yield Static(self.message, id="governor-message", markup=False)
            with Horizontal(id="governor-actions"):
                yield Button("Cancelar", id="governor-cancel")
                yield Button("Iniciar de todas formas", id="governor-continue", variant="warning")

    @on(Button.Pressed)
    def choose(self, event: Button.Pressed) -> None:
        self.dismiss(event.button.id == "governor-continue")

    def action_cancel(self) -> None:
        self.dismiss(False)
