from __future__ import annotations

import asyncio

from textual import work
from textual.widgets import Static

from ..core.bootstrap import configure_integrations, initialize_project, install_base_tools
from ..integrations.mcp_diagnostics import diagnose_project_mcp
from ..integrations.tools import check_tools
from ..workspace.manager import WorkspaceManager


class WorkspaceSetupActions:
    """Guided project bootstrap operations exposed by the TUI."""

    def action_init_preview(self) -> None:
        self._run_setup_action(
            "Vista previa de init",
            lambda: initialize_project(self.project, dry_run=True),
        )

    def action_init_project(self) -> None:
        self._run_setup_action("Proyecto inicializado", lambda: initialize_project(self.project))

    def action_init_minimal(self) -> None:
        self._run_setup_action(
            "Proyecto inicializado en modo mínimo",
            lambda: initialize_project(self.project, minimal=True),
        )

    def action_install_tools(self) -> None:
        self._run_setup_action("Herramientas base instaladas", install_base_tools)

    def action_integrate_clients(self) -> None:
        self._run_setup_action(
            "Integraciones MCP configuradas",
            lambda: configure_integrations(self.project),
        )

    def action_setup_all(self) -> None:
        def setup():
            results = [
                install_base_tools(),
                initialize_project(self.project),
                configure_integrations(self.project),
            ]
            combined = results[0]
            for result in results[1:]:
                combined.actions.extend(result.actions)
                combined.warnings.extend(result.warnings)
            combined.backup = results[1].backup
            return combined

        self._run_setup_action("Preparación completa finalizada", setup)

    def action_run_doctor(self) -> None:
        self._run_setup_action("Diagnóstico completado (solo lectura)", self._doctor_report)

    def _doctor_report(self) -> str:
        info = self.project
        rows = [
            f"Proyecto: {info.name}",
            f"Perfil: {info.profile_name}",
            "",
            "HERRAMIENTAS",
        ]
        rows.extend(
            f"{'✓' if item.installed else '!'} {item.name}: {item.command}"
            for item in check_tools()
        )
        rows.extend(("", "ARCHIVOS DEL PROYECTO"))
        for relative in ("AGENTS.md", "CLAUDE.md", ".ai/chxchx-tech.toml", ".ai/memory"):
            rows.append(f"{'✓' if (info.root / relative).exists() else '!'} {relative}")
        inspection = WorkspaceManager(info).inspect()
        rows.extend(("", "WORKSPACE"))
        if inspection.error:
            rows.append(f"✗ {inspection.error}")
        else:
            rows.append(f"✓ Configuración válida · confianza: {'sí' if inspection.trusted else 'no'}")
            rows.extend(f"! {warning}" for warning in inspection.warnings)
        rows.extend(("", "INTEGRACIONES MCP"))
        rows.extend(
            f"{item.status}: {item.client} / {item.server} · {item.detail}"
            for item in diagnose_project_mcp(info)
        )
        return "\n".join(rows)

    def _run_setup_action(self, title: str, action) -> None:
        if self._operation_pending or self._attach_pending:
            self.notify("Espera a que termine la acción actual", severity="warning")
            return
        if self._setup_pending:
            self.notify("Ya hay una tarea de configuración en curso", severity="warning")
            return
        self._setup_pending = True
        self._set_log(f"Ejecutando: {title}…")
        self.notify(f"Ejecutando: {title}…", severity="information")
        self._execute_setup_action(title, action)

    @work(group="setup")
    async def _execute_setup_action(self, title: str, action) -> None:
        try:
            result = await asyncio.to_thread(action)
        except Exception as exc:
            self._finish_setup_action(title, None, exc)
            return
        self._finish_setup_action(title, result, None)

    def _finish_setup_action(self, title: str, result, error: Exception | None) -> None:
        self._setup_pending = False
        if error is not None:
            exc = error
            message = f"Error: {exc}"
            self._query("#setup-output", Static).update(message)
            self._set_log(message)
            self.notify(str(exc), severity="error")
            return
        if isinstance(result, str):
            self._query("#setup-output", Static).update(result)
            self._set_log(title)
            self.notify(title, severity="information")
            return
        lines = []
        if result.backup:
            lines.append(f"Backup: {result.backup}")
        lines.extend(f"✓ {item}" for item in result.actions)
        lines.extend(f"! {item}" for item in result.warnings)
        detail = "\n".join(lines) or "No hay cambios pendientes."
        self._query("#setup-output", Static).update(detail)
        self._set_log(f"{title}: {len(result.actions)} acción(es), {len(result.warnings)} aviso(s)")
        self.notify(title, severity="warning" if result.warnings else "information")
        # Init and setup change the project state shown on Inicio. Refresh it
        # immediately so trust and workspace actions reflect the new config.
        self.refresh_dashboard()
        self._refresh_project_console()
