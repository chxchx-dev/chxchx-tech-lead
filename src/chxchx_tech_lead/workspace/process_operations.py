from __future__ import annotations

from .manager import WorkspaceInspection
from .process_manager import ProcessManager, ProcessManagerError
from .process_models import ManagedProcess, ProcessActionResult, ProcessStatus
from .service_models import WorkspaceOperationError


class WorkspaceProcessOperations:
    """Casos de uso que conectan la fachada con el administrador de procesos."""

    def process_statuses(self) -> list[ManagedProcess]:
        inspection = self.inspect()
        if inspection.config is None:
            return []
        return self._process_manager(inspection).list(persist=False)

    def start_project(self) -> list[ProcessActionResult]:
        inspection = self.inspect()
        if not inspection.trusted:
            raise WorkspaceOperationError(
                "Marca el proyecto como confiable antes de ejecutar su comando de desarrollo"
            )
        if inspection.config is None:
            raise WorkspaceOperationError("No hay una configuración válida para iniciar el proyecto")
        targets = list(inspection.config.processes)
        if not targets:
            raise WorkspaceOperationError(
                "No hay procesos configurados para iniciar. Ejecuta "
                "`chxchx-tech init` o configura workspace.processes en .ai/chxchx-tech.toml"
            )
        manager = self._process_manager(inspection)
        results: list[ProcessActionResult] = []
        for config in targets:
            try:
                results.append(manager.start(config.id))
            except ProcessManagerError as exc:
                raise WorkspaceOperationError(str(exc)) from exc
        return results

    def stop_project(self) -> list[ProcessActionResult]:
        inspection = self.inspect()
        if inspection.config is None:
            return []
        manager = self._process_manager(inspection)
        running = {
            process.id: process
            for process in manager.list()
            if process.status is ProcessStatus.RUNNING
        }
        results: list[ProcessActionResult] = []
        for config in inspection.config.processes:
            if config.id in running:
                try:
                    results.append(manager.stop(config.id))
                except ProcessManagerError as exc:
                    raise WorkspaceOperationError(str(exc)) from exc
        return results

    def read_process_log(self, process_id: str) -> str:
        inspection = self.inspect()
        if inspection.config is None:
            return ""
        try:
            return self._process_manager(inspection).read_log(process_id)
        except ProcessManagerError as exc:
            raise WorkspaceOperationError(str(exc)) from exc

    def _start_auto_processes(
        self,
        inspection: WorkspaceInspection,
        session: str,
        dry_run: bool,
        only_missing: bool = False,
    ) -> list[ProcessActionResult]:
        if not inspection.trusted or inspection.config is None:
            return []
        manager = self._process_manager(inspection)
        results = []
        for config in inspection.config.processes:
            if not config.auto_start:
                continue
            if only_missing:
                current = next((item for item in manager.list() if item.id == config.id), None)
                if current is not None and current.status is ProcessStatus.RUNNING:
                    continue
            try:
                results.append(manager.start(config.id, dry_run=dry_run))
            except ProcessManagerError as exc:
                raise WorkspaceOperationError(str(exc)) from exc
        return results

    def _process_manager(self, inspection: WorkspaceInspection) -> ProcessManager:
        return ProcessManager(
            self.project.root,
            inspection.config.processes if inspection.config is not None else (),
            trusted=inspection.trusted,
        )
