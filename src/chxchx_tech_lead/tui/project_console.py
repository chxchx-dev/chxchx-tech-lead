from __future__ import annotations

import shlex

from textual.widgets import DataTable, Static, TabbedContent

from ..workspace.service import WorkspaceOperationError


class WorkspaceProjectConsole:
    def _refresh_project_console(self) -> None:
        try:
            inspection = self.service.inspect()
            statuses = {item.id: item.status.value for item in self.service.process_statuses()}
        except (WorkspaceOperationError, OSError) as exc:
            self._query("#console-technology", Static).update(f"No se pudo inspeccionar el proyecto: {exc}")
            self._query("#console-processes", DataTable).clear()
            self._console_processes = {}
            self._console_process_id = None
            self._query("#console-output", Static).update("No hay comandos disponibles.")
            return

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
        tabs = self._query("#tabs", TabbedContent)
        if tabs.active != "console":
            return
        process_id = self._console_process_id
        config = self._console_processes.get(process_id) if process_id else None
        if config is None:
            self._query("#console-output", Static).update(
                "No hay procesos configurados para mostrar. Ejecuta chxchx-tech init o agrega "
                "un comando en .ai/chxchx-tech.toml."
            )
            return
        try:
            output = self.service.read_process_log(process_id)
        except (WorkspaceOperationError, OSError) as exc:
            output = f"No se pudo leer el log: {exc}"
        command = _command_text(config.command)
        working_dir = self.project.root / config.cwd
        self._query("#console-output", Static).update(
            f"{config.label} · comando configurado\n"
            f"$ {command}\nDirectorio: {working_dir}\n{'─' * 56}\n"
            f"{output or 'Esperando salida del proceso…'}"
        )

    def action_start_project(self) -> None:
        self._query("#tabs", TabbedContent).active = "console"
        try:
            results = self.service.start_project()
        except (WorkspaceOperationError, OSError) as exc:
            self._refresh_project_console()
            self._set_log(f"No se pudo iniciar el proyecto: {exc}")
            self.notify(str(exc), severity="error")
            return
        first_started = next(
            (result.process.id for result in results if result.changed),
            results[0].process.id if results else None,
        )
        if first_started:
            self._console_process_id = first_started
        self._refresh_project_console()
        message = "Proyecto iniciado" if results else "No hay procesos de inicio configurados"
        self._set_log(message)
        self.notify(message, severity="information")

    def action_stop_project(self) -> None:
        try:
            results = self.service.stop_project()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"No se pudo detener el proyecto: {exc}")
            self.notify(str(exc), severity="error")
            return
        self._refresh_project_console()
        message = f"Procesos detenidos: {len(results)}" if results else "El proyecto no tenía procesos activos"
        self._set_log(message)
        self.notify(message, severity="information")


def _command_text(command: list[str] | str) -> str:
    return command if isinstance(command, str) else shlex.join(command)
