from __future__ import annotations

import sys
from collections.abc import Sequence


NEW_CHAT_PROMPT = (
    "Inicia una conversación nueva y económica en contexto. Antes de responder, "
    "lee AGENTS.md si existe, .ai/PROJECT.md, .ai/CURRENT_STATE.md y "
    ".ai/HANDOFF.md. Recupera de Basic Memory, limitado al proyecto actual, "
    "las decisiones y notas relevantes para el estado y la tarea pendiente; "
    "no cargues ni reproduzcas conversaciones completas. Resume brevemente "
    "qué contexto persistido encontraste y continúa desde el pendiente. "
    "No supongas que información no guardada en esos archivos o en Basic Memory "
    "está disponible. Antes de responder al terminar trabajo sustancial, guarda "
    "sin pedir acción manual: actualiza estado y handoff y usa la herramienta "
    "write_memory de Basic Memory para conocimiento duradero del proyecto. "
    "No guardes saludos, preguntas triviales, secretos ni transcripciones completas."
)


def _supports_new_chat(command: Sequence[str]) -> bool:
    executable = command[0].replace("\\", "/").rsplit("/", 1)[-1].lower()
    if executable.endswith(".exe"):
        executable = executable[:-4]
    return executable in {"codex", "claude"}


def agent_pane_command(
    agent_id: str,
    command: Sequence[str],
    *,
    label: str,
    logo: str,
    new_chat: bool = False,
) -> list[str]:
    """Build the command used by both initial and dynamic agent panes."""
    if not command or any(not str(part).strip() for part in command):
        raise ValueError("El comando configurado del agente está vacío o contiene un argumento vacío")
    if new_chat and not _supports_new_chat(command):
        raise ValueError(
            f"El inicio de chat nuevo con recuperación de contexto solo está configurado para Codex y Claude: {command[0]}"
        )
    pane_command = [
        sys.executable,
        "-m",
        "chxchx_tech_lead.workspace.agent_pane",
        "--name",
        agent_id,
    ]
    if label:
        pane_command.extend(["--label", label])
    if logo:
        pane_command.extend(["--logo", logo])
    pane_command.extend(["--", *command])
    if new_chat:
        pane_command.append(NEW_CHAT_PROMPT)
    return pane_command
