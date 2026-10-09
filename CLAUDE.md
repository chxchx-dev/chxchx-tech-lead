@AGENTS.md

# Claude specific

- Usa las reglas de `AGENTS.md` como fuente principal del repositorio.
- Antes de cambios amplios, revisa `.ai/PROJECT.md`, `.ai/EDITOR-PARITY.md` y las pruebas relacionadas.
- En un proyecto preparado, lee `.ai/PROJECT.md` y `.ai/CURRENT_STATE.md` antes de proponer cambios estructurales.
- No reestructures el CLI completo para resolver cambios pequeños.
- Si una integración externa cambia, modifica el adapter correspondiente y documenta el cambio.
- Usa Serena para navegación semántica y Basic Memory para recuperar decisiones previas cuando estén disponibles.
- Al finalizar, resume validaciones, riesgos y pendientes en lugar de afirmar que una tarea está completa sin evidencia.

<!-- chxchx-tech:start claude-rules -->
# Claude specific

@AGENTS.md

- Usa Basic Memory para recuperar decisiones anteriores cuando la tarea dependa de contexto persistente.
- Antes de responder al terminar trabajo sustancial, persiste un checkpoint sin pedir una acción manual: usa `write_memory` para decisiones reutilizables y actualiza `.ai/CURRENT_STATE.md` / `.ai/HANDOFF.md` cuando reflejen el estado o pendientes actuales.
- No guardes saludos, preguntas triviales, secretos ni transcripciones completas; si no hubo cambio o conocimiento reutilizable, no crees una nota.
- Usa Serena para explorar símbolos y referencias antes de hacer búsquedas masivas por texto.
<!-- chxchx-tech:end claude-rules -->
