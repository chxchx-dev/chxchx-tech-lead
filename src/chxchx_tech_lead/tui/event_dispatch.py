"""Route Textual widget events to the TUI's focused, testable handlers."""

from __future__ import annotations

from textual.widgets import Button, DataTable, Input


def dispatch_button_pressed(app, event: Button.Pressed) -> None:
    button_id = event.button.id
    if not button_id or not button_id.startswith("btn-"):
        return
    method_name = "button_" + button_id.removeprefix("btn-").replace("-", "_")
    handler = getattr(app, method_name, None)
    if handler is not None:
        handler()
    # Restore app-level shortcuts after a mouse click focuses a button.
    app.set_focus(None)


def dispatch_input_changed(app, event: Input.Changed) -> None:
    handlers = {
        "chat-search": app.chat_search_changed,
        "memory-search": app.memory_search_changed,
        "skills-search": app.skills_search_changed,
    }
    handler = handlers.get(event.input.id)
    if handler is not None:
        handler(event)


def dispatch_table_row_highlighted(app, event: DataTable.RowHighlighted) -> None:
    handlers = {
        "chat-list": app.chat_row_highlighted,
        "errors-table": app.error_row_highlighted,
        "console-processes": app.console_process_highlighted,
        "memory-list": app.memory_row_highlighted,
        "agents-table": app.agent_row_highlighted,
        "skills-table": app.skill_row_highlighted,
        "packs-table": app.pack_row_highlighted,
    }
    handler = handlers.get(event.data_table.id)
    if handler is not None:
        handler(event)
