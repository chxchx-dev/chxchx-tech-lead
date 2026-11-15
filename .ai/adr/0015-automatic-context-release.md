# ADR 0015: Liberación automática de contexto en el chat visual

- Estado: Aceptada
- Fecha: 2026-11-07

## Contexto

Un contexto lleno hace más lentos y caros a los agentes. Cada CLI tiene su propia compactación y no es
controlable de forma homogénea desde Studio; además el usuario quiere que ocurra sola y sin perder el hilo.

## Decisión

- Python (`agent_chat`) decide, no Studio: tras un turno correcto y sin cancelar, si `tokens / ventana >= 0.90`
  (`chat_compaction.THRESHOLD`), se ejecuta un turno interno en la misma sesión, en solo lectura y sin eventos, que pide un
  traspaso (`HANDOFF_PROMPT`). Se guarda en `.ai/chat-handoffs/<proveedor>-<sesión>.md`.
- El siguiente `agent chat --session <vieja>` lo detecta (`plan_chat_turn`), arranca sin sesión y añade el traspaso, en un
  bloque `<archivo-de-contexto>` (la captura de memoria lo ignora), a los archivos de contexto de un chat nuevo. Studio
  adopta el id nuevo por el evento `session`, sin cambios en la interfaz salvo un aviso `notice`.
- La ventana viene del propio agente (Claude y Codex); para OpenCode se lee de `provider.*.models.*.limit.context` de su
  configuración o de `~/.cache/opencode/models.json`. Sin ventana conocida no se libera nada.
- No se usa la compactación nativa de cada CLI: su protocolo difiere y un traspaso propio es verificable de forma uniforme.

## Consecuencias

- Cada liberación cuesta un turno extra de lectura; el contexto nuevo parte del tamaño del traspaso.
- La calidad depende del resumen del agente; el límite es 24 000 caracteres.
- Si el traspaso falla, la conversación sigue en la sesión actual y se avisa.

## Actualización: persistencia al cerrar

- Cada chat (proyecto + agente) guarda `.ai/chat-state/<agente>.json` (`workspace/chat_state.py`): id de sesión y últimos 80 mensajes de texto
  (máx. 8 000 caracteres c/u; los que parecen secretos no se guardan). Se escribe al empezar el turno y al terminar, de forma atómica.
- Studio lo restaura al abrir el chat (`agent chat-state --restore`, `agent_chat_state.cpp`) y lo borra con «Nuevo chat» (`--reset`).
  `--restore` marca el estado como «sin verificar»: el siguiente turno comprueba una vez si el proveedor conserva la sesión
  (`chat_resume.py`; Claude/Codex por sus transcripciones, OpenCode con `session list`). Si no, la conversación guardada viaja como
  bloque `<archivo-de-contexto>` en el primer mensaje de una sesión nueva. Si no se puede saber, se asume que existe.
- El traspaso sigue ligado al id de la sesión vieja: como Studio restaura ese id, reabrir antes del siguiente mensaje lo usa; se borra
  cuando la sesión nueva ya existe.
- El diario `SESSIONS.md` y la memoria explícita no cambian: siguen sirviendo a los chats nuevos; el estado del chat solo continúa el mismo hilo.
