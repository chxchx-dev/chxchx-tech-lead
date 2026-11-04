# ADR-0012: Asistente de Git con los agentes del proyecto

- Estado: Aceptada
- Fecha: 2026-11-02

## Contexto

El panel Git (ADR 0011) es claro para quien conoce Git, pero redactar buenos mensajes
de commit y resolver ramas separadas o conflictos sigue siendo lo difícil. Studio ya
integra agentes (Codex, Claude Code, OpenCode); usarlos aquí evita añadir otro
proveedor, pero implica enviar código a un servicio externo.

## Decisión

- **Dos capas.** (1) Una guía **por reglas** (`guidanceFor`) decide el siguiente paso solo con el
  estado del repositorio: funciona sin agente, al instante y sin enviar nada. (2) El asistente
  (agente) redacta mensajes y da consejo cuando la persona lo pide.
- **Solo propone.** Un mensaje sugerido se escribe en la caja y la persona decide; el consejo se
  muestra en un diálogo y se puede copiar. Nada se prepara, guarda, fusiona ni ejecuta por su cuenta.
- **Consulta puntual (`agent chat --lean`)**: sin el contexto del proyecto, memoria ni diario,
  siempre en solo lectura (aunque se pida otro modo), sin guardar la sesión (`--ephemeral` en Codex,
  `--no-session-persistence` en Claude; la sesión de OpenCode se borra al terminar) y sin tocar
  `.ai/`. Sigue pasando por el trust del proyecto. `--prompt-file` evita el límite de la línea de
  comandos con diferencias grandes; Studio escribe el prompt en un archivo temporal y lo elimina.
- **Qué se envía**: un resumen del estado, los commits recientes (para imitar el estilo) y, para un
  mensaje, la diferencia de lo preparado (o de todo lo pendiente si nada lo está, con los archivos
  nuevos como adiciones), recortada a ~48 000 caracteres. Para un conflicto, los bloques
  `<<<<<<<`…`>>>>>>>` de los archivos en conflicto (acotados). Nunca el árbol completo.
- **Privacidad**: los archivos sensibles (`.env*`, llaves y certificados, `credentials*`,
  `secrets*`, `.ssh/`, `.aws/`…) se omiten por completo y se enmascaran valores que parezcan
  secretos (claves con nombre, tokens conocidos, bloques de clave privada). Antes de la primera consulta
  con cada agente se pide confirmación, con opción de no volver a preguntar.
- **Qué agente**: el del chat que está abierto; si no, el elegido antes en la tarjeta; si no, el
  primero configurado que Studio sabe manejar. Se puede cambiar desde la tarjeta.
- **Modelo ligero por defecto**: una pregunta corta no necesita el modelo grande del agente. La
  consulta pasa `--model` y `--effort` (que `agent chat` ya aceptaba). El modo *automático* elige,
  de la lista que publica el propio agente (`bridge agent-options`), el primer modelo que parece
  ligero por su nombre (`mini`, `nano`, `haiku`, `flash`, `lite`, `small`…) o por su tamaño
  (modelos locales de hasta 14b), con el esfuerzo más bajo que ese modelo admite (`low` o
  `minimal`). Haiku no recibe esfuerzo. Si nada parece ligero, se usa el modelo del agente con
  esfuerzo bajo cuando se sabe que lo admite. La persona puede elegir cualquier modelo de la lista
  (o escribirlo en la configuración del agente) o el predeterminado del agente, por agente. Si la
  lista no se puede leer, decide el agente. No se inventan nombres de modelos.
- **Estado fresco**: si hay conflictos, se consulta en el momento de preguntar si la fusión está a
  medias, para no construir la consulta sobre un estado viejo.
- **Interfaz**: tarjeta de guía con tono (informativo, aviso, al día), botón *Sugerir* junto al de
  guardar, botón *Pedir consejo* también en los avisos de error, y cancelación en curso.

## Consecuencias

- Los mensajes y consejos dependen de la calidad del agente y de que el proyecto sea de confianza;
  si no hay agente, la tarjeta sigue guiando y explica cómo conectarlo.
- El enmascarado es heurístico: reduce el riesgo, no lo elimina. La confirmación previa existe
  por eso.
- "Ligero" se decide por nombre o tamaño, no por precio ni calidad reales: un modelo que no encaje
  en esos patrones no se elige solo, pero se puede elegir a mano.
- Los flags `--ephemeral` y `--no-session-persistence` y el formato del borrado de sesiones de
  OpenCode no se han comprobado con las CLI reales (no estaban instaladas).
- Fuera de alcance: resolver conflictos automáticamente, preparar archivos por la persona, y
  sugerir cómo dividir un cambio en varios commits más allá de la guía.
