from __future__ import annotations

import asyncio
from pathlib import Path

from textual import work
from textual.widgets import DataTable, Input, Static

from ..integrations.ci_service import ci_logs
from ..integrations.deploy_service import (
    deployment_status_payload,
    remove_deployment,
    run_deployment,
    setup_deployment,
)
from ..integrations.github_cli import GitHubCliError
from .ci_prompt import CiConfirmation


class DeployActions:
    def button_deploy_refresh_action(self) -> None:
        self._refresh_deploy()

    def _refresh_deploy(self) -> None:
        self._load_deploy_status(self.project)

    def button_deploy_setup_action(self) -> None:
        if self._deploy_pending:
            return
        root = self.project.root.resolve()
        branch = self._query("#deploy-branch", Input).value.strip() or None
        try:
            result, plan = setup_deployment(root, branch, dry_run=True)
        except (OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        mode = f"automático al subir cambios a {branch}" if branch else "manual por defecto"
        message = (
            f"Proyecto: {self.project.name}\nRuntime: Docker Compose ({plan.compose_file})\n"
            f"Modo: {mode}\nWorkflow: {result.path}\n\n"
            "El workflow se conectará por SSH al VPS y desplegará el commit seleccionado. "
            "Usa un usuario dedicado sin acceso root; prepara allí el clon limpio del repositorio, "
            "Docker Compose y el archivo .env fuera de Git.\n\n"
            "Configura en el Environment production de GitHub las variables "
            f"{', '.join(plan.required_variables)} y los secretos {', '.join(plan.required_secrets)}. "
            "Verifica la huella SSH del VPS antes de guardar VPS_SSH_KNOWN_HOSTS."
        )
        if not result.changed:
            self.notify("El workflow de CD ya está actualizado", severity="information")
            self._load_deploy_status(self.project)
            return
        self._deploy_pending = "setup"
        self._deploy_target_root = root
        self._deploy_target_branch = branch
        self.push_screen(
            CiConfirmation("Previsualización de CD al VPS", message, "Guardar workflow"),
            self._finish_deploy_setup,
        )

    def _finish_deploy_setup(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_deploy_pending()
            self._set_log("Setup de CD cancelado; no se escribió ningún archivo.")
            return
        self._write_deploy_workflow(self._deploy_target_root, self._deploy_target_branch)

    @work(group="deploy-write", exclusive=True)
    async def _write_deploy_workflow(self, root: Path | None, branch: str | None) -> None:
        if root is None:
            self._clear_deploy_pending()
            return
        try:
            result, _plan = await asyncio.to_thread(setup_deployment, root, branch)
        except (OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        finally:
            self._clear_deploy_pending()
        backup = f"\nBackup: {result.backup}" if result.backup else ""
        self._set_log(
            f"Workflow CD guardado: {result.path}{backup}\nConfigura las variables y secretos de GitHub para activarlo."
        )
        self.notify("Workflow CD guardado", severity="information")
        self._load_deploy_status(self.project)

    def button_deploy_run_action(self) -> None:
        if self._deploy_pending:
            return
        root = self.project.root.resolve()
        try:
            preview = run_deployment(root, confirmed=True, dry_run=True)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        self._deploy_pending = "run"
        self._deploy_target_root = root
        self.push_screen(
            CiConfirmation(
                "Confirmar despliegue en el VPS",
                f"Proyecto: {self.project.name}\nComando: {' '.join(preview.command)}\n\n"
                "Se enviará el commit al VPS y se ejecutará Docker Compose. "
                "El workflow intentará restaurar el commit anterior si el despliegue o su health check falla.",
                "Desplegar",
            ),
            self._finish_deploy_run,
        )

    def _finish_deploy_run(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_deploy_pending()
            self._set_log("Despliegue cancelado; no se contactó GitHub ni el VPS.")
            return
        self._dispatch_deploy_run(self._deploy_target_root)

    @work(group="deploy-run", exclusive=True)
    async def _dispatch_deploy_run(self, root: Path | None) -> None:
        if root is None:
            self._clear_deploy_pending()
            return
        try:
            result = await asyncio.to_thread(run_deployment, root, confirmed=True)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        finally:
            self._clear_deploy_pending()
        self._set_log(result.stdout or "Despliegue solicitado a GitHub Actions.")
        self.notify("Despliegue solicitado", severity="information")
        self._load_deploy_status(self.project)

    def button_deploy_remove_action(self) -> None:
        if self._deploy_pending:
            return
        root = self.project.root.resolve()
        try:
            preview = remove_deployment(root, dry_run=True)
        except (OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        if not preview.changed:
            self.notify("No hay workflow de CD administrado para retirar", severity="information")
            return
        self._deploy_pending = "remove"
        self._deploy_target_root = root
        self.push_screen(
            CiConfirmation(
                "Retirar CD del VPS",
                f"Archivo: {preview.path}\n\nSe guardará un backup local y se retirará el workflow. "
                "Esto no detiene servicios ni borra datos del VPS.",
                "Retirar workflow",
            ),
            self._finish_deploy_remove,
        )

    def _finish_deploy_remove(self, confirmed: bool | None) -> None:
        if not confirmed:
            self._clear_deploy_pending()
            self._set_log("Retirada de CD cancelada; el workflow sigue intacto.")
            return
        self._remove_deploy_workflow(self._deploy_target_root)

    @work(group="deploy-remove", exclusive=True)
    async def _remove_deploy_workflow(self, root: Path | None) -> None:
        if root is None:
            self._clear_deploy_pending()
            return
        try:
            result = await asyncio.to_thread(remove_deployment, root)
        except (OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        finally:
            self._clear_deploy_pending()
        self._set_log(f"Workflow CD retirado. Backup: {result.backup}")
        self.notify("Workflow CD retirado", severity="information")
        self._load_deploy_status(self.project)

    def button_deploy_logs_action(self) -> None:
        run_id = self._deploy_selected_run
        if not run_id:
            self.notify("Selecciona primero una ejecución de CD", severity="warning")
            return
        self._read_deploy_logs(self.project.root.resolve(), run_id)

    def deploy_row_highlighted(self, event) -> None:
        self._deploy_selected_run = str(event.row_key.value)

    @work(group="deploy-logs", exclusive=True)
    async def _read_deploy_logs(self, root: Path, run_id: str) -> None:
        try:
            result = await asyncio.to_thread(ci_logs, root, run_id)
        except (GitHubCliError, OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        self._query("#deploy-detail", Static).update(result.stdout or "La ejecución no contiene logs fallidos.")

    @work(group="deploy-refresh", exclusive=True)
    async def _load_deploy_status(self, project) -> None:
        try:
            payload = await asyncio.to_thread(deployment_status_payload, project.root)
        except (OSError, ValueError) as exc:
            self._show_deploy_error(exc)
            return
        if project is not self.project:
            return
        self._query("#deploy-branch", Input).value = payload["auto_deploy_branch"] or ""
        table = self._query("#deploy-runs", DataTable)
        table.clear()
        for run in payload["recent_runs"]:
            run_id = str(run.get("databaseId", ""))
            if not run_id:
                continue
            table.add_row(
                run_id,
                str(run.get("headBranch", "")),
                str(run.get("status", "")),
                str(run.get("conclusion", "")),
                key=run_id,
            )
        mode = f"automático en {payload['auto_deploy_branch']}" if payload["auto_deploy_branch"] else "manual"
        summary = (
            f"{self.project.name} · {'configurado' if payload['managed'] and payload['integrity_ok'] else 'desactivado o requiere revisión'}\n"
            f"Runtime: {payload['runtime'] or 'Compose no detectado'} ({payload['compose_file'] or 'sin manifiesto'}) · {mode}\n"
            f"GitHub CLI: {'conectado' if payload['authenticated'] else 'sin sesión'}\n"
            f"Variables: {', '.join(payload['required_variables'])} · Secretos: {', '.join(payload['required_secrets'])}"
        )
        if payload["message"]:
            summary += f"\n{payload['message']}"
        if payload["warnings"]:
            summary += "\nAvisos: " + " · ".join(payload["warnings"])
        self._query("#deploy-summary", Static).update(summary)

    def _show_deploy_error(self, exc: Exception) -> None:
        self._clear_deploy_pending()
        message = f"Error de CD: {exc}"
        self._query("#deploy-summary", Static).update(message)
        self._set_log(message)
        self.notify(str(exc), severity="error")

    def _clear_deploy_pending(self) -> None:
        self._deploy_pending = ""
        self._deploy_target_root = None
        self._deploy_target_branch = None
