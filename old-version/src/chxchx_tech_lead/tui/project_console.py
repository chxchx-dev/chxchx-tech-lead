from __future__ import annotations

import asyncio
import shlex

from textual import work
from textual.widgets import DataTable, Static, TabbedContent

from ..workspace.service import WorkspaceOperationError


class WorkspaceProjectConsole:
    def _refresh_project_console(self) -> None:
        if not self._project_console_active():
            return
        if self._console_pending:
            self._console_refresh_again = True
            return
        self._console_pending = True
        self._load_project_console(self.project, self.service)

    @work(group="project-console", exclusive=True)
    async def _load_project_console(self, project, service) -> None:
        try:
            inspection, process_statuses = await asyncio.to_thread(
                lambda: (service.inspect(), service.process_statuses())
            )
        except Exception as exc:
            self._finish_project_console(project, service, None, None, exc)
            return
        self._finish_project_console(project, service, inspection, process_statuses, None)

    def _finish_project_console(self, project, service, inspection, process_statuses, error) -> None:
        self._console_pending = False
        refresh_again = self._console_refresh_again
        self._console_refresh_again = False
        if project is not self.project or service is not self.service:
            self._refresh_project_console()
            return
        if refresh_again:
            self._refresh_project_console()
            return
        if error is not None:
            self._query("#console-technology", Static).update(
                f"No se pudo inspeccionar el proyecto: {type(error).__name__}: {error}"
            )
            self._query("#console-processes", DataTable).clear()
            self._console_processes = {}
            self._console_process_id = None
            self._query("#console-output", Static).update("No hay comandos disponibles.")
            return

        statuses = {item.id: item.status.value for item in process_statuses}
        technologies = [
            *self.project.stacks,
            *self.project.languages,
            *self.project.package_managers,
        ]
        technology_text = " · ".join(dict.fromkeys(technologies)) or "tecnología no detectada"
        trust_text = "confiable" if inspection.trusted else "requiere confianza"
        self._query("#console-technology", Static).update(
            f"{self.project.name} · {technology_text} · {trust_text}"
        )

        self._console_processes = {
            item.id: item
            for item in inspection.config.processes
        } if inspection.config is not None else {}
        table = self._query("#console-processes", DataTable)
        table.clear()
        process_ids = list(self._console_processes)
        for process_id, config in self._console_processes.items():
            command = _command_text(config.command)
            mode = "Sí" if config.auto_start else "No"
            table.add_row(process_id, command, mode, statuses.get(process_id, "STOPPED"), key=process_id)

        if self._console_process_id not in self._console_processes:
            self._console_process_id = process_ids[0] if process_ids else None
        selected_row = process_ids.index(self._console_process_id) if self._console_process_id else None
        if selected_row is not None:
            table.move_cursor(row=selected_row)
        self._refresh_project_output()

    def _select_console_process(self, process_id: str) -> None:
        if process_id in self._console_processes:
            self._console_process_id = process_id
            self._refresh_project_output()

    def _refresh_project_output(self) -> None:
        if self._console_output_pending:
            return
        if not self._project_console_active():
            return
        process_id = self._console_process_id
        config = self._console_processes.get(process_id) if process_id else None
        if config is None:
            self._query("#console-output", Static).update(
                "No hay procesos configurados para mostrar. Ejecuta chxchx-tech init o agrega "
                "un comando en .ai/chxchx-tech.toml."
            )
            return
        self._console_output_pending = True
        self._load_project_output(self.project, self.service, process_id, config)

    @work(group="console-output", exclusive=True)
    async def _load_project_output(self, project, service, process_id, config) -> None:
        try:
            output = await asyncio.to_thread(service.read_process_log, process_id)
        except (WorkspaceOperationError, OSError) as exc:
            output = f"No se pudo leer el log: {exc}"
        self._console_output_pending = False
        if (
            project is not self.project
            or service is not self.service
            or process_id != self._console_process_id
            or not self._project_console_active()
        ):
            return
        command = _command_text(config.command)
        working_dir = self.project.root / config.cwd
        self._query("#console-output", Static).update(
            f"{config.label} · comando configurado\n"
            f"$ {command}\nDirectorio: {working_dir}\n{'─' * 56}\n"
            f"{output or 'Esperando salida del proceso…'}"
        )

    def _project_console_active(self) -> bool:
        return (
            self._query("#tabs", TabbedContent).active == "work"
            and self._query("#work-tabs", TabbedContent).active == "console"
        )

    def action_start_project(self) -> None:
        self._show("console")

        def started(results) -> None:
            first_started = next(
                (result.process.id for result in results if result.changed),
                results[0].process.id if results else None,
            )
            if first_started:
                self._console_process_id = first_started
            self._refresh_project_console()

        self._perform(
            "Proyecto iniciado",
            self.service.start_project,
            refresh=True,
            on_success=started,
        )

    def action_stop_project(self) -> None:
        def stopped(results) -> None:
            self._refresh_project_console()
            message = (
                f"Procesos detenidos: {len(results)}"
                if results
                else "El proyecto no tenía procesos activos"
            )
            self._set_log(message)
            self.notify(message, severity="information")

        self._perform("Proyecto detenido", self.service.stop_project, refresh=True, on_success=stopped)


def _command_text(command: list[str] | str) -> str:
    return command if isinstance(command, str) else shlex.join(command)
