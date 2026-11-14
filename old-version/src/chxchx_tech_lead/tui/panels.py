from __future__ import annotations

import asyncio
from pathlib import Path

from textual import work
from textual.widgets import DataTable, Input, Static, TabbedContent

from ..core.registry import load_registry
from ..integrations.chat_history import list_conversations
from ..workspace.error_cache import error_cache_path, list_errors, record_error
from ..workspace.memory_history import list_memory_notes
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
        self.refresh_dashboard()

    def _render_agents(self, statuses) -> None:
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
                key=agent.id,
            )

    def _refresh_handoff(self) -> None:
        path = self.project.root / ".ai" / "HANDOFF.md"
        if not path.exists():
            self._query("#handoff", Static).update(
                f"No existe todavía:\n{path}\n\nUsa el botón para generarlo."
            )
            return
        try:
            content = path.read_text(encoding="utf-8")
        except OSError as exc:
            self._query("#handoff", Static).update(f"No se pudo leer handoff: {exc}")
            return
        self._query("#handoff", Static).update(content)

    def _refresh_conversations(self) -> None:
        if self._chat_search_timer is not None:
            self._chat_search_timer.stop()
            self._chat_search_timer = None
        self._conversation_refresh_generation += 1
        generation = self._conversation_refresh_generation
        project = self.project
        query = self._query("#chat-search", Input).value
        self._load_conversations(project, generation, query)

    @work(group="conversation-refresh", exclusive=True)
    async def _load_conversations(self, project, generation: int, query: str) -> None:
        try:
            conversations = await asyncio.to_thread(
                list_conversations,
                project.root,
                query=query,
            )
        except (OSError, ValueError) as exc:
            if generation == self._conversation_refresh_generation and project is self.project:
                self._query("#chat-summary", Static).update(
                    f"No se pudo leer el historial local: {exc}"
                )
            return
        if generation != self._conversation_refresh_generation or project is not self.project:
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
        if self._memory_search_timer is not None:
            self._memory_search_timer.stop()
            self._memory_search_timer = None
        self._memory_refresh_generation += 1
        generation = self._memory_refresh_generation
        project = self.project
        query = self._query("#memory-search", Input).value
        self._load_memory_history(project, generation, query)

    @work(group="memory-refresh", exclusive=True)
    async def _load_memory_history(self, project, generation: int, query: str) -> None:
        try:
            notes = await asyncio.to_thread(list_memory_notes, project.root, query=query)
        except (OSError, ValueError) as exc:
            if generation == self._memory_refresh_generation and project is self.project:
                self._query("#memory-summary", Static).update(
                    f"No se pudo leer la memoria local: {exc}"
                )
            return
        if generation != self._memory_refresh_generation or project is not self.project:
            return
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

        memory_path = project.root / ".ai" / "memory"
        if not memory_path.is_dir():
            message = f"Este proyecto todavía no tiene memoria local:\n{memory_path}"
        elif not notes:
            message = "No hay notas que coincidan con la búsqueda."
        else:
            message = f"{len(notes)} nota(s) de {project.name} · {memory_path}"
        self._query("#memory-summary", Static).update(message)
        self._query("#memory-detail", Static).update(
            "Selecciona una nota para ver su contenido."
        )

    def _refresh_errors(self) -> None:
        table = self._query("#errors-table", DataTable)
        table.clear()
        self._cached_errors = {}
        try:
            errors = list_errors(project_path=self.project.root)
        except OSError as exc:
            self._query("#errors-summary", Static).update(f"No se pudo leer la caché: {exc}")
            return
        for index, error in enumerate(errors):
            key = f"{index}:{error.occurred_at}"
            self._cached_errors[key] = error
            table.add_row(
                error.occurred_at.replace("T", " ")[:19],
                error.project,
                error.operation,
                error.message.replace("\n", " ")[:100],
                key=key,
            )
        cache_path = error_cache_path()
        summary = (
            f"{len(errors)} error(es) de {self.project.name} · caché: {cache_path}"
            if errors
            else f"No hay errores guardados para {self.project.name} · caché: {cache_path}"
        )
        self._query("#errors-summary", Static).update(summary)
        self._query("#error-detail", Static).update("Selecciona un error para ver el detalle.")

    def _set_log(self, message: str) -> None:
        self._query("#log", Static).update(message)
        normalized = message.casefold()
        if normalized.startswith((
            "error:", "error al ", "error ", "no se pudo ", "no pude ", "falló ", "fallo "
        )):
            try:
                record_error(
                    project=self.project.name,
                    project_path=self.project.root,
                    operation="TUI",
                    message=message,
                )
            except OSError:
                pass
            try:
                if (
                    self._query("#tabs", TabbedContent).active == "more"
                    and self._query("#more-tabs", TabbedContent).active == "errors"
                ):
                    self._refresh_errors()
            except Exception:
                pass
