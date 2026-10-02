from __future__ import annotations

from textual.widgets import Input

from ..workspace.service import WorkspaceOperationError


class WorkspaceTerminalActions:
    """Suspend and restore the TUI around interactive terminal adapters."""

    def action_attach_workspace(self) -> None:
        if self._attach_pending:
            return
        self._attach_pending = True
        message = "Entrando a Zellij… Para volver al TUI: Ctrl+O y después D."
        self._set_log(message)
        self.notify(message, severity="information")
        self.set_timer(0.1, self._attach_workspace_now)

    def action_attach_agents_workspace(self) -> None:
        # Recover an EXITED/missing session before trying to attach.
        self.action_start_workspace_all()

    def action_open_terminal(self) -> None:
        if self._attach_pending:
            return
        self._attach_pending = True
        message = "Abriendo terminal paralela en Zellij… Para volver a la TUI: Ctrl+O y después D."
        self._set_log(message)
        self.notify(message, severity="information")
        self.set_timer(0.1, self._open_terminal_now)

    def _open_terminal_now(self) -> None:
        self._attach_pending = False
        try:
            with self.suspend():
                self.service.open_terminal()
        except Exception as exc:
            self._set_log(f"Error: {type(exc).__name__}: {exc}")
            self.notify(str(exc), severity="error")
            self._restore_after_external_terminal()
            return
        message = "Terminal abierta; regresaste a la TUI. La pane sigue disponible en Zellij."
        self._set_log(message)
        self.notify(message, severity="information")
        self._restore_after_external_terminal()

    def action_attach_agent_terminal(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Selecciona un agente de la tabla antes de abrir su terminal")
            self.notify("Falta seleccionar un agente", severity="warning")
            return
        if self._attach_pending:
            return
        self._attach_pending = True
        message = f"Abriendo la terminal de {agent_id}… Para volver a la TUI: Ctrl+O y después D."
        self._set_log(message)
        self.notify(message, severity="information")
        self.set_timer(0.1, lambda: self._attach_agent_terminal_now(agent_id))

    def _attach_agent_terminal_now(self, agent_id: str) -> None:
        try:
            # Las comprobaciones y el arranque ocurren antes de ceder el TTY.
            self.service.prepare_agents()
            with self.suspend():
                self.service.attach_agent(agent_id)
        except Exception as exc:
            self._set_log(f"Error: {type(exc).__name__}: {exc}")
            self.notify(str(exc), severity="error")
            self._attach_pending = False
            self._refresh_agents()
            self._restore_after_external_terminal()
            return
        message = f"Terminal de {agent_id} cerrada; regresaste a la TUI."
        self._set_log(message)
        self.notify(message, severity="information")
        self._attach_pending = False
        self._restore_after_external_terminal()

    def _attach_workspace_now(self) -> None:
        self._attach_pending = False
        try:
            with self.suspend():
                self.service.attach()
        except (WorkspaceOperationError, OSError) as exc:
            self._set_log(f"Error: {exc}")
            self.notify(str(exc), severity="error")
            self._restore_after_external_terminal()
            return
        message = "Zellij finalizado; regresaste al TUI. La sesión sigue disponible."
        self._set_log(message)
        self.notify(message, severity="information")
        self._restore_after_external_terminal()

    def _restore_after_external_terminal(self) -> None:
        self.refresh_dashboard()
        self.refresh(repaint=True, layout=True)
        self.set_timer(0.05, lambda: self.refresh(repaint=True, layout=True))
