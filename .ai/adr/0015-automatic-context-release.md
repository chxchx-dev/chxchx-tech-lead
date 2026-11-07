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
