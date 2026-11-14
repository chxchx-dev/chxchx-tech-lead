from __future__ import annotations

from collections.abc import Callable, Sequence

from ..workspace.resources import ResourceManager, format_bytes
from .governor_prompt import GovernorPrompt


class ResourceGovernorActions:
    def _guard_agent_launch(
        self,
        target_ids: Sequence[str] | None,
        launch: Callable[[], None],
        *,
        new_chat: bool = False,
    ) -> None:
        if self._governor_pending:
            self.notify("Ya se está revisando el presupuesto de recursos", severity="warning")
            return
        try:
            inspection = self.service.inspect()
        except Exception:
            launch()
            return
        config = inspection.config
        if config is None:
            launch()
            return

        configured = {agent.id for agent in config.agents}
        targets = configured if target_ids is None else configured.intersection(target_ids)
        if not targets:
            launch()
            return
        active = {
            agent.id for agent in self._last_agents
            if agent.pane == "RUNNING" and agent.id in configured
        }
        planned = len(targets) if new_chat else len(targets - active)
        resources = ResourceManager(config.resources)
        snapshot = resources.system()
        warnings = resources.agent_start_warnings(len(active), planned, snapshot)
        if not warnings:
            launch()
            return

        usage = (
            f"RAM: {snapshot.memory_percent:.0f}% usada, "
            f"{format_bytes(snapshot.available_bytes)} disponible de "
            f"{format_bytes(snapshot.total_bytes)}.\n"
            if snapshot.total_bytes > 0 else "Uso de RAM: no disponible.\n"
        )
        message = (
            f"Proyecto: {self.project.name}\n{usage}"
            f"Agentes activos detectados: {len(active)}; nuevos previstos: {planned}; "
            f"límite: {config.resources.max_agents}.\n\n"
            + "\n".join(f"• {warning}" for warning in warnings)
            + "\n\nChxChx no detendrá procesos automáticamente."
        )
        self._governor_pending = True
        self._set_log("RAM Governor: confirma antes de iniciar agentes con recursos limitados.")
        self.push_screen(
            GovernorPrompt(message),
            lambda proceed: self._finish_governed_launch(proceed, launch),
        )

    def _finish_governed_launch(self, proceed: bool | None, launch: Callable[[], None]) -> None:
        self._governor_pending = False
        if proceed:
            launch()
            return
        self._set_log("Inicio de agentes cancelado por el usuario.")
        self.notify("Inicio cancelado; no se detuvo ningún proceso", severity="information")
        if self._dashboard_refresh_again and not self._dashboard_pending:
            self.refresh_dashboard()
