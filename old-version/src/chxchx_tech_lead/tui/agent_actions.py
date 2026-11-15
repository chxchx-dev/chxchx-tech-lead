from __future__ import annotations

from textual.widgets import Input


class WorkspaceAgentActions:
    def action_start_agents(self) -> None:
        self._guard_agent_launch(
            None,
            lambda: self._perform("Agentes iniciados", lambda: self.service.start_agents()),
        )

    def action_start_agent_selected(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe un ID de agente, por ejemplo `codex` o `claude`")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._guard_agent_launch(
            (agent_id,),
            lambda: self._perform(
                f"Agente `{agent_id}` iniciado",
                lambda: self.service.start_agent(agent_id),
            ),
        )

    def action_start_new_chat(self) -> None:
        agent_id = self._query("#agent-id", Input).value.strip()
        if not agent_id:
            self._set_log("Escribe codex o claude para iniciar un chat nuevo con contexto")
            self.notify("Falta el ID del agente", severity="warning")
            return
        self._guard_agent_launch(
            (agent_id,),
            lambda: self._perform(
                f"Chat nuevo de `{agent_id}` iniciado con contexto persistido",
                lambda: self.service.start_agent(agent_id, new_chat=True),
            ),
            new_chat=True,
        )
