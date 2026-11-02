# ADR-0009: Chat visual con Codex y Claude mediante turnos estructurados

- Estado: Aceptada
- Fecha: 2026-11-01

## Contexto

En Studio, Codex y Claude Code solo funcionaban como CLIs interactivas en una
pestaña PTY. Para que Studio pueda sustituir al editor del usuario hace falta
una conversación nativa: mensajes legibles, comandos con su salida, cambios de
archivos e historial. Los dos agentes tienen modos oficiales legibles por
máquina: `codex exec --json` y `claude -p --output-format stream-json`. Codex
también ofrece un `app-server` JSON-RPC, todavía experimental.

## Decisión

- Cada mensaje del usuario ejecuta un turno: `chxchx-tech agent chat AGENTE
  --prompt … [--session ID] --mode read-only|workspace-write`. Ese turno lanza
  una vez el modo estructurado del agente, que reanuda la sesión anterior. El
  prompt se envía por stdin; así se evita citar texto libre en los shims
  `.cmd` de Windows.
- El adapter `integrations/agent_stream.py` convierte los eventos de cada
  proveedor al contrato JSONL `chxchx.agent-event` v1 (session, message,
  reasoning, tool, usage, notice, error, start, done). Studio solo pinta ese
  contrato y no interpreta los formatos de Codex o Claude.
- Python sigue siendo la autoridad. Trust obligatorio, preflight de RAM con
  confirmación en Studio, contexto del chat nuevo (compactación, memoria y
  diario) y captura de memoria/diario al terminar: es la misma lógica que usa
  la pestaña de terminal (`workspace/chat_context.py`).
- Permisos: "Solo lectura" usa el sandbox `read-only` de Codex y `plan` en
  Claude. "Editar workspace" usa `workspace-write` y `acceptEdits`.
  En este modo no hay aprobación interactiva por acción.
- Para cancelar, Studio escribe `cancel` (o cierra stdin). El CLI detiene el
  árbol de procesos del agente: grupo POSIX, o `taskkill /T` en Windows. Cerrar
  la pestaña cancela el turno.
- La pestaña PTY se mantiene como alternativa completa y para OpenCode.
- Modelo y effort: `bridge agent-options` (`chxchx.agent-options` v1) entrega
  disponibilidad, comando de instalación, valores por defecto y opciones. En
  Codex las lee de `models_cache.json` y `config.toml`; en Claude usa los alias
  documentados y `settings.json`. El turno pasa `-m`/`-c
  model_reasoning_effort` en Codex y `--model`/`--effort` en Claude.
- Uso: el evento `limits` informa del contexto usado y la ventana. En Codex
  añade los límites de 5 h y semanal, leídos del rollout del hilo; en Claude
  sale del `result`, y su costo llega en `usage`.
- Imágenes: se copian a `.ai/chat-attachments/`. Codex las recibe con
  `--image` (validado en vivo, también en `resume`). Claude recibe sus rutas en
  el prompt para leerlas con Read, porque el formato de imágenes por stdin no
  está documentado.

## Consecuencias

- Hay un proceso por turno: Codex lee de nuevo el contexto de la sesión en
  cada mensaje. Con el caché de tokens se reduce el coste, pero no la
  latencia de arranque.
- Aún no hay aprobación de cada comando o diff. Para lograrlo habría que
  pasar a `codex app-server` y a `--permission-prompt-tool` de Claude, cuando
  esas interfaces se estabilicen.
- Codex se validó en Fedora con la versión 0.166. Claude se implementó según
  su formato `stream-json` documentado y falta validarlo con la CLI real.
