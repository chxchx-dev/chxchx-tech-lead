# Handoff

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

## Estado tras la 0.5.1 (estable publicada, `dev` en 0.5.2.dev0)

- Publicado `v0.5.1` (11 assets, manifiesto estable). Trae chat en vivo de los tres agentes, niveles de acceso, 22 Mods para Claude/Codex/OpenCode, plan de commits por tema en Git y el instalador de Windows ARM64/x86 corregido.
- Probar a mano en Studio con agentes reales: chat de OpenCode (el fallo «dejó de responder» se atribuyó a que esperar tu respuesta contaba como cuelgue; no se reprodujo en el real), la terminal integrada de OpenCode (¿responde libvterm a las consultas de terminal de OpenCode al arrancar?), el plan de commits en un repo con temas mezclados y la pantalla de Mods.
- Mods: `.claude/skills/` y `.agents/skills/` no se versionan; `mod apply` los regenera. `chxchx-model-advisor` solo sirve con Claude.
- CI: estuvo rojo desde el primer commit del chat en vivo por pruebas dependientes de plataforma (UTF-8 en Windows, `getfqdn` lento en macOS, pruebas TUI con pausas fijas). Revisar el CI tras cada push. Quedan pruebas TUI con `pause(0.x)` fijas que podrían ser inestables.
- Pendiente: `--add-dir`, firma de instaladores (secretos), ARM64 nativo de Windows sin ejecutar en hardware real, probar `install.ps1`/Setup.exe en Windows real.

## Instaladores y releases (publicados: 0.5.2 estable y preview dev)

- Windows: x64 nativo; ARM64 usa el paquete x64 (emulación) salvo que el manifiesto traiga `asset.windows-arm64`; x86 de 32 bits cae a solo CLI. El job `native-windows-arm64` (experimental, compilación cruzada, nunca ejecutado) sube un artefacto que NO va al release: probarlo en un ARM64 real y luego conectarlo a `stage-release` (el instalador y `release_assets.py` ya lo soportan). La `0.5.1` estable debe llevar el arreglo de arquitectura de `install.ps1`.

- Empujar a `dev` y revisar que pasen `installer-smoke` (sobre todo Windows `install.ps1`, que nunca se ejecutó, y macOS DMG), `native-build` con el `.rc`/`.icns` nuevos y NSIS con `/DCHANNEL`. Arreglar lo que aparezca antes de anunciar los comandos.
- Primer release estable: subir `pyproject.toml` a `X.Y.Z` (sin `.dev0`), mantener `uv.lock` al día, empujar el tag `vX.Y.Z`. Comprobar que `releases/latest/download/install.sh` y `.ps1` responden.
- Cuando haya certificados: crear los secretos (`WINDOWS_CERT_PFX_BASE64`, `WINDOWS_CERT_PASSWORD`, `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`, `MACOS_SIGN_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`) y probar `sign-windows.ps1`/`sign-macos.sh` (nunca ejecutados). La firma de macOS recrea el DMG, así que revisa su aspecto.
- Pendiente menor: `install.sh` no copia el icono si el paquete es anterior al `.png` nuevo; el updater de Studio no actualiza el icono/`.desktop`; Arch/openSUSE no tienen paquete.

## Asistente de Git (hecho; validar con agentes reales)

- Comprobar qué modelo elige el modo automático con cada CLI real (`opencode models`, catálogo de Codex, alias de Claude) y que `--model`/`--effort low` no den error; si el "ligero" elegido no convence, ajustar los patrones de `looksLight` o fijar el modelo desde el menú.

- Probar con un agente real: sugerir mensaje con y sin cambios preparados, consejo ante ramas separadas / envío rechazado / conflictos, cancelar, cambiar de agente, y que el diálogo de consentimiento aparezca la primera vez. Comprobar que ninguna consulta deja sesión (Codex `--ephemeral`, Claude `--no-session-persistence`, OpenCode borra la suya) ni toca `.ai/`.
- Revisar con un repo que tenga `.env` y secretos en el código que nada sensible sale (ver ADR 0012). Pendiente: resolver conflictos con ayuda del agente en el editor, sugerir cómo dividir un cambio en varios commits.

## Visor de Git (fases 1 y 2 hechas; probar contra un remoto real)

- Probar a mano con un remoto real (GitHub por HTTPS con gestor de credenciales y por SSH con llave): publicar, enviar, recibir, divergencia + fusión, conflicto + cancelar fusión, y que un remoto sin credenciales falle rápido con el mensaje de ayuda (sin colgar). Probar también cancelar una operación lenta, Apartar/aplicar/eliminar y el color del explorador en un repo grande.
- Pendiente (fase 3): preparar por bloques, blame, historial por archivo, resolver conflictos con un editor, búsqueda en el historial. Decisión tomada: no ofrecer `rebase` ni envío forzado.

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
