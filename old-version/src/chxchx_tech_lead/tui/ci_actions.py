from __future__ import annotations

import asyncio
from pathlib import Path

from textual import work
from textual.widgets import DataTable, Static

from ..integrations.ci_service import ci_logs, ci_status_payload, remove_ci, run_ci, setup_ci
from ..integrations.github_cli import GitHubCliError
from .ci_prompt import CiConfirmation


class CiActions:
    def button_ci_refresh(self) -> None:
        self._refresh_ci()

    def button_ci_setup_action(self) -> None:
        if self._ci_pending:
            return
        root = self.project.root.resolve()
        try:
            result, plan = setup_ci(root, dry_run=True)
        except (OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        message = (
            f"Proyecto: {self.project.name}\nStacks: {', '.join(plan.stacks)}\n"
            f"Archivo: {result.path}\n\nEl workflow corre en push, pull request y ejecución manual. "
            "Solo solicita contents: read y no incluye secretos."
        )
        if plan.warnings:
            message += "\n\nAvisos:\n" + "\n".join(plan.warnings)
        if not result.changed:
            self.notify("El workflow administrado ya está actualizado", severity="information")
            self._refresh_ci()
            return
        self._ci_pending = "setup"
        self._ci_target_root = root
        self.push_screen(
            CiConfirmation("Previsualización del pipeline", message, "Guardar workflow"),
            self._finish_ci_setup,
        )

    def _finish_ci_setup(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_ci_pending()
            self._set_log("Setup de CI cancelado; no se escribió ningún archivo.")
            return
        self._write_ci_workflow(self._ci_target_root)

    @work(group="ci-write", exclusive=True)
    async def _write_ci_workflow(self, root: Path | None) -> None:
        if root is None:
            self._clear_ci_pending()
            return
        try:
            result, _plan = await asyncio.to_thread(setup_ci, root)
        except (OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        finally:
            self._clear_ci_pending()
        backup = f"\nBackup: {result.backup}" if result.backup else ""
        self._set_log(f"Workflow CI guardado: {result.path}{backup}\nSúbelo a GitHub para activarlo.")
        self.notify("Workflow CI guardado", severity="information")
        self._refresh_ci()

    def button_ci_run_action(self) -> None:
        if self._ci_pending:
            return
        root = self.project.root.resolve()
        try:
            preview = run_ci(root, dry_run=True)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        self._ci_pending = "run"
        self._ci_target_root = root
        self.push_screen(
            CiConfirmation(
                "Ejecutar CI en GitHub",
                f"Proyecto: {self.project.name}\nComando: {' '.join(preview.command)}\n\n"
                "Esto iniciará una ejecución remota; el resultado puede consumir minutos de runner.",
                "Ejecutar CI",
            ),
            self._finish_ci_run,
        )

    def _finish_ci_run(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_ci_pending()
            self._set_log("Ejecución de CI cancelada; no se contactó GitHub.")
            return
        self._dispatch_ci_run(self._ci_target_root)

    @work(group="ci-run", exclusive=True)
    async def _dispatch_ci_run(self, root: Path | None) -> None:
        if root is None:
            self._clear_ci_pending()
            return
        try:
            result = await asyncio.to_thread(run_ci, root)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        finally:
            self._clear_ci_pending()
        self._set_log(result.stdout or "CI solicitado a GitHub.")
        self.notify("Ejecución de CI solicitada", severity="information")
        self._refresh_ci()

    def button_ci_remove_action(self) -> None:
        if self._ci_pending:
            return
        root = self.project.root.resolve()
        try:
            preview = remove_ci(root, dry_run=True)
        except (OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        if not preview.changed:
            self.notify("No hay workflow administrado para retirar", severity="information")
            return
        self._ci_pending = "remove"
        self._ci_target_root = root
        self.push_screen(
            CiConfirmation(
                "Retirar CI administrado",
                f"Se retirará {preview.path}. ChxChx guardará un backup local antes de borrarlo.",
                "Retirar workflow",
            ),
            self._finish_ci_remove,
        )

    def _finish_ci_remove(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_ci_pending()
            self._set_log("Retirada de CI cancelada; el workflow sigue intacto.")
            return
        self._remove_ci_workflow(self._ci_target_root)

    @work(group="ci-remove", exclusive=True)
    async def _remove_ci_workflow(self, root: Path | None) -> None:
        if root is None:
            self._clear_ci_pending()
            return
        try:
            result = await asyncio.to_thread(remove_ci, root)
        except (OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        finally:
            self._clear_ci_pending()
        self._set_log(f"Workflow retirado. Backup: {result.backup}")
        self.notify("Workflow retirado con backup", severity="information")
        self._refresh_ci()

    def button_ci_logs_action(self) -> None:
        run_id = self._ci_selected_run
        if not run_id:
            self.notify("Selecciona primero una ejecución", severity="warning")
            return
        self._read_ci_logs(self.project.root.resolve(), run_id)

    def ci_row_highlighted(self, event) -> None:
        self._ci_selected_run = str(event.row_key.value)

    @work(group="ci-logs", exclusive=True)
    async def _read_ci_logs(self, root: Path, run_id: str) -> None:
        try:
            result = await asyncio.to_thread(ci_logs, root, run_id)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        self._query("#ci-detail", Static).update(result.stdout or "No hay logs fallidos.")

    def _refresh_ci(self) -> None:
        self._load_ci_status(self.project)

    @work(group="ci-refresh", exclusive=True)
    async def _load_ci_status(self, project) -> None:
        try:
            payload = await asyncio.to_thread(ci_status_payload, project.root)
        except (OSError, ValueError) as exc:
            self._show_ci_error(exc)
            return
        if project is not self.project:
            return
        table = self._query("#ci-runs", DataTable)
        table.clear()
        for run in payload["recent_runs"]:
            run_id = str(run.get("databaseId", ""))
            if not run_id:
                continue
            table.add_row(
                run_id,
                str(run.get("workflowName", "")),
                str(run.get("headBranch", "")),
                str(run.get("status", "")),
                str(run.get("conclusion", "")),
                key=run_id,
            )
        state = "configurado" if payload["managed"] and payload["integrity_ok"] else "desactivado o requiere revisión"
        summary = (
            f"{self.project.name} · {state} · GitHub CLI "
            f"{'conectado' if payload['authenticated'] else 'sin sesión'}\n"
            f"Stacks detectados: {', '.join(payload['supported_stacks']) or 'ninguno'}"
        )
        if payload["message"]:
            summary += f"\n{payload['message']}"
        if payload["warnings"]:
            summary += "\nAvisos: " + " · ".join(payload["warnings"])
        self._query("#ci-summary", Static).update(summary)

    def _show_ci_error(self, exc: Exception) -> None:
        self._set_log(f"Error CI/CD: {exc}")
        self.notify(str(exc), severity="error")

    def _clear_ci_pending(self) -> None:
        self._ci_pending = None
        self._ci_target_root = None
