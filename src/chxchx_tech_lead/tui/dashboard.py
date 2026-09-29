from __future__ import annotations

from pathlib import Path

from textual.widgets import DataTable, Static, TabbedContent

from ..core.models import ProjectInfo
from ..core.registry import load_registry
from ..workspace.process_manager import ProcessManager
from ..workspace.resources import ResourceManager, format_bytes, summarize_process_resources
from ..workspace.service import WorkspaceOperationError, WorkspaceService


class WorkspaceDashboard:
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
                f"Lenguajes: {', '.join(self.project.languages) or 'sin detectar'}  |  "
                f"Gestor: {', '.join(self.project.package_managers) or 'sin detectar'}\n"
                f"Arranque: {self._startup_summary(inspection)}\n"
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


