from __future__ import annotations

import asyncio

from textual import work
from textual.widgets import Button, DataTable, Static, TabbedContent

from ..workspace.resources import ResourceManager, format_bytes, summarize_process_resources
from ..workspace.service import WorkspaceOperationError
from .dashboard_data import AggregatedResourceRow, DashboardSnapshot, collect_dashboard


class WorkspaceDashboard:
    def refresh_dashboard(self) -> None:
        # The command palette is a modal screen. Its widgets replace the
        # main screen while it is open, so wait for the dismiss callback.
        if self._dashboard_pending:
            self._dashboard_refresh_again = True
            return
        if (
            self._palette_open
            or self._operation_pending
            or self._setup_pending
            or self._attach_pending
            or self._start_pending
        ):
            return
        self._dashboard_refresh_again = False
        include_aggregated_resources = (
            self._query("#tabs", TabbedContent).active == "more"
            and self._query("#more-tabs", TabbedContent).active == "resources"
        )
        self._dashboard_pending = True
        self._collect_dashboard_data(
            self.project,
            self.service,
            include_aggregated_resources,
        )

    @work(group="dashboard-refresh", exclusive=True)
    async def _collect_dashboard_data(self, project, service, include_aggregated_resources) -> None:
        try:
            snapshot = await asyncio.to_thread(
                collect_dashboard,
                project,
                service,
                include_aggregated_resources=include_aggregated_resources,
            )
        except Exception as exc:
            self._finish_dashboard_refresh(project, service, None, exc)
            return
        self._finish_dashboard_refresh(project, service, snapshot, None)

    def _finish_dashboard_refresh(
        self,
        project,
        service,
        snapshot: DashboardSnapshot | None,
        error: Exception | None,
    ) -> None:
        self._dashboard_pending = False
        refresh_again = self._dashboard_refresh_again
        self._dashboard_refresh_again = False
        if self._palette_open:
            self._dashboard_refresh_again = True
            return
        if project is not self.project or service is not self.service:
            self.refresh_dashboard()
            return
        if error is not None:
            self._set_log(f"Error al actualizar el panel: {type(error).__name__}: {error}")
            if refresh_again:
                self.refresh_dashboard()
            return
        if snapshot is not None:
            self._render_dashboard(snapshot)
            if refresh_again or (
                snapshot.aggregated_resources is None and self._resources_view_active()
            ):
                self.refresh_dashboard()

    def _resources_view_active(self) -> bool:
        return (
            self._query("#tabs", TabbedContent).active == "more"
            and self._query("#more-tabs", TabbedContent).active == "resources"
        )

    def _render_dashboard(self, snapshot: DashboardSnapshot) -> None:
        try:
            inspection = snapshot.inspection
            self._last_inspection = inspection
            summary = self._query("#summary", Static)
            self._query("#btn-trust", Button).disabled = inspection.trusted
            self._query("#btn-console-trust", Button).disabled = inspection.trusted
            summary.update(
                f"Proyecto: {self.project.name}  |  Perfil: {self.project.profile_name}\n"
                f"Workspace: {inspection.state.status.value}  |  Trust: {'sí' if inspection.trusted else 'no'}\n"
                f"Stack: {', '.join(self.project.stacks) or 'sin detectar'}\n"
                f"Lenguajes: {', '.join(self.project.languages) or 'sin detectar'}  |  "
                f"Gestor: {', '.join(self.project.package_managers) or 'sin detectar'}\n"
                f"Arranque: {self._startup_summary(inspection)}\n"
                f"Ruta: {self.project.root}"
            )
            if snapshot.aggregated_resources is not None:
                self._render_aggregated_resources(snapshot.aggregated_resources)
            if snapshot.agent_statuses is not None:
                self._render_agents(snapshot.agent_statuses)
            elif snapshot.agent_status_error is not None:
                self._set_log(f"Agentes: {snapshot.agent_status_error}")
            if inspection.config is None:
                self._set_log("No hay configuración válida de workspace")
                return
            resources = ResourceManager(inspection.config.resources)
            system = snapshot.system
            if system is None:
                self._set_log("No se pudieron leer los recursos del sistema")
                return
            self._update_resources(resources, system)
            process_metrics = snapshot.process_metrics
            self._update_project_resources(process_metrics, system.total_bytes)
            self._refresh_process_table(snapshot.processes, process_metrics, "#overview-processes")
            self._refresh_process_table(snapshot.processes, process_metrics, "#processes-table")
            self._refresh_projects()
            self._refresh_handoff()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")

    def _startup_summary(self, inspection) -> str:
        if inspection.config is None or not inspection.config.processes:
            return "sin comando sugerido; revisa .ai/chxchx-tech.toml"
        return " · ".join(
            f"{item.id}: {' '.join(item.command)} ({'al iniciar' if item.auto_start else 'manual'})"
            for item in inspection.config.processes
        )

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

    def _render_aggregated_resources(self, rows: list[AggregatedResourceRow]) -> None:
        table = self._query("#resources-all-projects", DataTable)
        table.clear()
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
