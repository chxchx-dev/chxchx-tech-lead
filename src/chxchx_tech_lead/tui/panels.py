from __future__ import annotations

from pathlib import Path

from textual.widgets import DataTable, Input, Static

from ..core.registry import load_registry
from ..integrations.chat_history import list_conversations
from ..workspace.autosave_status import diagnose_autosave, format_autosave_status
from ..workspace.memory_history import list_memory_notes
from ..workspace.service import WorkspaceOperationError
from ..workspace.state import load_state


class WorkspacePanels:
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

    def _refresh_conversations(self) -> None:
        query = self._query("#chat-search", Input).value
        try:
            conversations = list_conversations(self.project.root, query=query)
        except (OSError, ValueError) as exc:
            self._query("#chat-summary", Static).update(f"No se pudo leer el historial local: {exc}")
            return
        table = self._query("#chat-list", DataTable)
        table.clear()
        self._conversations = {}
        for conversation in conversations:
            key = f"{conversation.provider}:{conversation.session_id}"
            self._conversations[key] = conversation
            table.add_row(
                conversation.provider,
                conversation.modified_at.strftime("%Y-%m-%d %H:%M"),
                conversation.title,
                conversation.preview,
                key=key,
            )
        if conversations:
            message = f"{len(conversations)} conversación(es) de {self.project.name} · Codex y Claude Code"
        else:
            message = "No hay conversaciones locales para este proyecto o no coinciden con la búsqueda."
        self._query("#chat-summary", Static).update(message)
        self._query("#chat-detail", Static).update("Selecciona una conversación para leerla.")

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

