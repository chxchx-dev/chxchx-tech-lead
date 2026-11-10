# Handoff

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Abrir carpeta de proyecto (2026-11-09)

- Implementado sin commit en `dev`. Pendiente: probar a mano el diálogo y la ventana nueva (Fedora, macOS, Windows; en macOS el ejecutable dentro del .app debe relanzarse bien), añadir smoke Qt si se quiere cobertura y documentar el atajo en README/EDITOR-PARITY. `main_window.cpp` sigue sobre 300 líneas (excepción previa).
- Tip de entorno: con `LD_LIBRARY_PATH` apuntando a la instalación stable de Studio, el build falla con `Qt_6.11 not found`; compilar con `env -u LD_LIBRARY_PATH`.

## Release 0.5.3 publicado (2026-11-08)

- Versión estable y lock sincronizados. Validación local: 511 passed, 1 skipped, 10 deselected; Studio y smoke nativo compilados, smoke nativo y `--version` correctos. El CI del tag pasó todas las matrices/builds y `installer-smoke` en Ubuntu, Windows y macOS; el release `v0.5.3` se publicó con 11 assets. Su primer intento reveló el ajuste de Rich en Ubuntu/Python 3.13, corregido con `soft_wrap=True`.
- Pendiente fuera de CI: probar el botón ✦ desde una ventana de Studio con un modelo de visión real.

## Imágenes pegadas y caché de descripciones (2026-11-08)

- Implementado **✦ Describir y guardar** a demanda con el agente/modelo actual en `--lean`. Genera Markdown con OCR, distribución, observaciones, inferencias e incertidumbres; lo guarda por SHA-256, caduca a los 30 días y limita a 200 entradas. Al readjuntar, el Markdown se usa por defecto; mientras el original esté disponible, el botón permite volver a enviarlo. La copia temporal del portapapeles y las copias de staging de consultas `--lean` se eliminan solo tras completar el guardado/turno; los archivos originales elegidos por el usuario se conservan.
- Smoke nativo cubre generación simulada, reutilización, vencimiento y eliminación solo de copias del portapapeles; la suite Python pasó (511 passed, 1 skipped, 10 deselected), `compileall` y el build completo de Studio también pasan. Pendiente: probar el botón con un agente/modelo de visión real en Studio y revisar la caché bajo `.ai/cache/image-descriptions/`.

## OpenCode y registro de errores (2026-11-08)

- Causa confirmada del mensaje mostrado: `agent add` necesita `.ai/chxchx-tech.toml`; sin ese archivo OpenCode todavía no está registrado en el workspace.
- Para habilitarlo en el proyecto afectado: `chxchx-tech init <PATH> --dry-run`, revisar y repetir sin `--dry-run`; luego `chxchx-tech agent add opencode --path <PATH>` o usar Añadir agente en Studio, y volver a confiar el proyecto.
- Errores de alta, preflight y chat quedan en la caché local. Consultar con `chxchx-tech workspace errors <PATH>` o `Más → Errores` en TUI/Studio. Suite completa: 511 passed, 1 skipped, 10 deselected.

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Chat visual: aprobaciones en vivo (Claude, Codex y OpenCode hechos)

- Hecho con Claude (ADR 0014): permisos y `AskUserQuestion` en vivo. Probar a mano en Studio con el Claude real: aprobar, «en este chat», denegar, preguntas con varias opciones/selección múltiple, detener mientras espera, y cerrar la pestaña con una solicitud abierta.
- Resuelto: el contexto del chat nuevo se inyecta en el prompt (`inline_project_context`), así que ya no hay permiso para leerlo. Los archivos grandes se recortan (12 KB c/u, 36 KB en total).
- Hecho con Codex (`app-server`): probar a mano en Studio con el Codex real (comando fuera del sandbox, archivo fuera del proyecto, denegar, pregunta `request_user_input`), y vigilar cambios del protocolo experimental. El proceso es por turno; un proceso por chat permitiría «permitir en este chat».
- Hecho con OpenCode (`serve`): probar a mano en Studio con el OpenCode real (aprobar, «en este chat» con patrón, denegar —detiene el turno—, pregunta, cancelar con una solicitud abierta) y comprobar que no quedan procesos `opencode serve` tras cerrar la pestaña.
- Pendiente: carpetas adicionales (`--add-dir`). El reintento automático de la fase 1 ya conserva el borrador del cuadro de texto.

## OpenCode 2.x y contexto automático (publicados en la 0.5.2; dev sigue en 0.5.3.dev0)

- OpenCode 2.x: el modo en vivo usa `agent_opencode_live2.py` (API `/api/...`, spec en `GET /openapi.json` del servidor); 1.x sigue con `agent_opencode_live.py`. Los turnos sin servidor (asistente de Git, CLI) en 2.x usan `run --standalone` (`agent_opencode_turn.py`). Pendiente: probar en Studio con la 2.x real, y subagentes/permisos de sesiones hijas con la 2.x.
- Liberación automática de contexto al 90 % (ADR 0015): probada con Claude, Codex y OpenCode reales forzando el umbral; falta probarla en Studio con una conversación larga de verdad.
- Búsqueda de agentes fuera del PATH (`executable_search.py`): hipótesis para «no inicia» en otros PC; la causa probable en instalaciones nuevas era además OpenCode 2.x. Confirmar en la Mac.

## Último cambio

- El test de recuperación del ProcessManager usa un handle simulado. Cubre persistencia, recuperación y selección del terminador sin iniciar un subprocess de larga duración, eliminando la espera de `taskkill` en Windows.
- El test de recuperación del ProcessManager ahora parchea el terminador correspondiente a la plataforma. En Windows evitaba un bloqueo al no ejecutar `taskkill` sobre el proceso largo de prueba.
- Los tests de ciclo de vida de la TUI esperan la finalización de acciones, dashboard y consola con un límite de 5 segundos, en vez de usar sleeps de 200–400 ms. Esto evita cerrar la app de prueba antes de que Textual procese el worker, como ocurrió en CI de Windows/Python 3.11 y 3.12.
- Corregido el eje del layout `Agentes`: encabezado y panel de uso ocupan el ancho; la orientación solo organiza los agentes.
- `agents_tab_layout` ahora genera el formato pane-only que espera `zellij action new-tab --layout`, y el adapter pasa el nombre con `--name`.
- La pestaña y los panes se crean desde el worker de attach después de que el cliente conecte. El worker repite el foco por un intervalo corto porque una acción antes del attach no selecciona la pestaña para el cliente nuevo.
- `prepare_agents` ya no crea panes dinámicos dentro de la última pestaña restaurada. Al abrir un agente, el attach enfoca la pestaña y luego el pane solicitado.
- Corregido `--root` del panel de uso. README actualizado sobre el layout y el diálogo de confianza propio de Codex/Claude.
- Se conservan el acceso directo `A` y el arreglo de foco por ID para panes de agentes.

## Pendiente

- El usuario debe salir de la TUI abierta, ejecutar `chxchx-tech tui .` y pulsar `A`.
- Si ya tiene una pestaña `Agentes` creada con el layout anterior, recrear esa sesión una vez con `chxchx-tech run . --recreate`. Claude Code puede pedir que se confirme su propia confianza de carpeta.

## Validación

- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/workspace/test_process_manager.py` (13 passed).
- `python -m compileall src tests scripts` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/workspace/test_process_manager.py` (13 passed).
- `python -m compileall src tests scripts` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `python -m compileall src` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/tui/test_lifecycle.py` (16 passed).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `UV_CACHE_DIR=/tmp/chxchx-uv-cache uv run --no-sync python -m compileall src`
- `UV_CACHE_DIR=/tmp/chxchx-uv-cache uv run --no-sync pytest` (165 passed, 1 skipped).
- Zellij real, sesión existente con Terminales: el attach agregó `Agentes`, mostró 4 panes (header 2×164, uso 8×164, agentes 27×82), imprimió contenido en ambos y enfocó Codex.
- Inicio real de las CLIs configuradas: Codex mostró su TUI y Claude pidió confirmar la confianza de la carpeta. Las sesiones de smoke test se eliminaron.
- `git diff --check`.
