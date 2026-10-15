from __future__ import annotations

from textual import on
from textual.app import ComposeResult
from textual.containers import Container, Horizontal
from textual.screen import ModalScreen
from textual.widgets import Button, Static


class CiConfirmation(ModalScreen[bool]):
    """Confirm a CI workflow change or an explicit GitHub dispatch."""

    CSS = """
    CiConfirmation { align: center middle; background: #1d1f29 85%; }
    #ci-confirm-dialog {
        width: 76; height: auto; max-height: 80%; padding: 1 2;
        background: #2e303d; border: solid #d6bd82;
    }
    #ci-confirm-title { color: #f1d99b; text-style: bold; padding-bottom: 1; }
    #ci-confirm-message { height: auto; max-height: 16; padding-bottom: 1; }
    #ci-confirm-actions { height: auto; align-horizontal: right; }
    #ci-confirm-actions Button { width: auto; margin-left: 1; }
    """

    BINDINGS = [("escape", "cancel", "Cancelar")]

    def __init__(self, title: str, message: str, confirm_label: str) -> None:
        super().__init__()
        self.title = title
        self.message = message
        self.confirm_label = confirm_label

    def compose(self) -> ComposeResult:
        with Container(id="ci-confirm-dialog"):
            yield Static(self.title, id="ci-confirm-title")
            yield Static(self.message, id="ci-confirm-message", markup=False)
            with Horizontal(id="ci-confirm-actions"):
                yield Button("Cancelar", id="ci-confirm-cancel")
                yield Button(self.confirm_label, id="ci-confirm-accept", variant="warning")

    @on(Button.Pressed)
    def choose(self, event: Button.Pressed) -> None:
        self.dismiss(event.button.id == "ci-confirm-accept")

    def action_cancel(self) -> None:
        self.dismiss(False)
