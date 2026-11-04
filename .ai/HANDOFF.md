# Handoff

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Instaladores y releases (hecho en Linux; validar el resto en CI)

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

## Visor de Git (fase 1 hecha; probar a mano)

- Probar en Studio con un repo real: abrir la pestaña Git, preparar/quitar/descartar (revisar `.git/chxchx-backups/`), commit con y sin nada preparado, historial en un repo con muchas ramas, cambiar/crear/borrar ramas, un repo sin commits, una carpeta que no es repo, y un repo grande (rendimiento del estado y del historial).
- Confirmar en el CI de Windows y macOS que `native-git-smoke` y `native-git-ui-smoke` pasan (rutas, `GIT_CONFIG_GLOBAL`, fin de línea).
- Fase 2: `push`/`pull`/`fetch` con autenticación (sin pedir contraseñas en la interfaz), stash y estado de Git en el árbol de archivos. Fase 3: preparar por bloques, blame, historial por archivo, resolver conflictos y búsqueda en el historial.

## Selector con OpenCode y capturas (hecho; revisar a mano)

- Probar en Studio: Inicio → selector → `＋ Añadir OpenCode` → Añadir OpenCode: ver la vista previa, confirmar, volver a dar trust y comprobar que `opencode` aparece como agente.
- Las capturas del README son de ejemplo; si cambia la UI, repetirlas con las variables de `tests/README.md` y revisar que no se cuele nada real.

## Validación en vivo: lista de pruebas

- Ejecutar `uv run pytest -m live -v -s` con OpenCode y Ollama instalados (pasos y variables en `tests/README.md`). Las pruebas dicen qué adaptador ajustar si fallan; la de solo lectura (`read_only_mode_really_blocks…`) es la de seguridad.
- Revisar la salida de `an_ask_permission_without_a_terminal…` (caracterización) y guardar la salida cruda con `CHXCHX_LIVE_CAPTURE` para renovar fixtures.
- Confirmar en CI de Windows que las E2E con ejecutables falsos (shims `.cmd`) pasan; si no, ajustar `tests/fakes/fake_clis.py`.

## Modelos locales con Ollama (pestaña de Studio hecha; falta validar en vivo)

- Hecho: backend, CLI y pestaña de Studio (ver `.ai/CURRENT_STATE.md` y ADR 0010).
- Siguiente: probar en un escritorio con Ollama instalado: abrir la pestaña, ver los estados (sin Ollama, detenido, listo), vista previa, descarga real con progreso y cancelación, y que `ollama/<modelo>-ctx32k` aparezca en el chat visual de OpenCode.
- Validar con Ollama real: `/api/pull`, `/api/tags`, `ollama create` con Modelfile y llamadas a herramientas de OpenCode. Comprobar las claves `limit.context`/`limit.output` del proveedor, tomadas de un resumen de la documentación.
- Hecho: acceso desde Inicio, `local-models remove` y modelos cargados (`/api/ps`). Al validar en vivo, comprobar también el borrado real, que el botón de Inicio se vea bien y que `ollama.loaded` refleje los modelos en memoria.
- Pendiente menor: acceso desde el área Agentes (requiere ampliar su barra de 4 acciones), descargar modelos fuera del catálogo y revisar el catálogo cuando cambie la biblioteca de Ollama.

## OpenCode como tercer agente (pendiente de validar en vivo)

- Hecho: chat visual, catálogo, diario de sesiones y Studio para OpenCode (detalle en `.ai/CURRENT_STATE.md` y ADR 0009).
- Primer paso al retomar: instalar OpenCode (`npm install -g opencode-ai`), autenticar un proveedor y correr `chxchx-tech agent add opencode` + `chxchx-tech agent chat opencode --prompt hola`. Confirmar que (1) el prompt entra por stdin, (2) los campos de los eventos coinciden (`sessionID`, `part.text`, `part.state`, `part.tokens`), (3) `OPENCODE_PERMISSION` se respeta en solo lectura y (4) qué hace un permiso `ask` sin terminal. Si el prompt por stdin falla, pasarlo como argumento posicional.
- Fuera de alcance por ahora: historial de conversaciones de Studio para OpenCode, ventana de contexto y niveles de effort, `ollama launch opencode` en el chat visual.
- Sin commit: los cambios están en el árbol de trabajo de `dev`. Basic Memory no está disponible en las herramientas de esta sesión.

## README y merge de ramas

- Se integró `main` en `dev` (`cd86d46`) y luego `feature/reliable-chat-memory` (`9ceb081`); los 10 commits de esa rama quedan conservados en `dev`.
- Se actualizó la cabecera del README con el arte ASCII solicitado, badges de CI/Python/Qt/plataformas y navegación rápida. Se conservaron las guías, comandos y detalles que ya estaban documentados.
- Se publicó `dev` en `origin` con el commit `5eae006`; las ramas local y remota `feature/reliable-chat-memory` se eliminaron después de integrar sus 10 commits.
- Verificación: `git diff --check` sin errores. No se ejecutaron tests en esta ronda.
- Basic Memory no está disponible en las herramientas de esta sesión; el checkpoint quedó en `.ai/CURRENT_STATE.md` y este archivo.

## Instalación local de ChxChx Studio

- El release oficial `studio-dev-preview` respondió 404 al `--dry-run`; no había paquete publicado para Fedora. Se compiló el código de `dev` (`5eae006`) y se instaló en `~/.local/opt/chxchx-studio/dev-5eae006`.
- Se creó `~/.local/bin/chxchx-studio-dev` y el acceso de menú `~/.local/share/applications/chxchx-studio-dev.desktop`. El launcher configura los plugins Qt del sistema y usa el CLI del `.venv` de este checkout.
- Arranque verificado con captura offscreen, `ldd` sin dependencias faltantes y `desktop-file-validate`. No se ejecutó CTest ni pytest. Esta instalación local depende de que se mantenga el checkout en su ruta actual.

## Responsivo y pendientes menores

- Hecho: lista de secciones estable (acciones movidas a la barra del área), cuadrículas que se adaptan al ancho, páginas con desplazamiento, chat en paneles estrechos e iconos por tipo de archivo. También títulos Markdown completos, paleta ANSI para el tema claro y líneas partidas con sangría.
- Validación: CTest `8/8`; pytest `291 passed, 1 skipped`; capturas a 1024×700, 1280×800 y 1920×1080, y chat a 460/560/1000 px. Antes, la ventana no bajaba de 814 px de alto ni el chat de 724 px de ancho.
- Pendiente: probar a mano cómo se arrastran los separadores de paneles y el comportamiento en Windows/macOS. La terminal en tema claro no se verificó con un programa real (sin pantalla aquí).

## Rediseño visual, temas y coloreado de código

- Hecho: motor de temas, iconos propios, hoja de estilos moderna, coloreado semántico, selector de apariencia y barra de estado. Capturas offscreen revisadas en Medianoche, Claro, Aurora y Bosque (inicio, editor Python/C++/Markdown/CMake, diálogo de Apariencia, chat).
- Validación: CTest `7/7`; pytest `290 passed, 1 skipped`; build sin warnings; Claude real validado por CLI.
- Corrección: la paleta de comandos es una lista fija y antes no incluía el chat visual (la documentación decía lo contrario). Ahora sí lo incluye.
- Con el uso real se vio que la compactación automática y el diario funcionan. También salió un bug: Codex registra AGENTS.md como primer mensaje del usuario y el diario lo tomaba como "Inicio". Corregido y con prueba.
- Pendiente: QA con mouse (menús, hover, diálogo) y en Windows/macOS. Los encabezados Markdown solo colorean el `#`. Faltan iconos de tipo de archivo en el explorador. La terminal hereda la paleta, pero sus colores ANSI no se adaptan al tema claro.

## Chat visual con estilo de IDE

- Hecho: rediseño completo de la vista del chat con widgets nativos y el tema de Studio; capturas offscreen revisadas (bienvenida, turno real de Codex con imagen y código, Claude no instalado).
- Validación: CTest `7/7` (el smoke cubre segmentación de markdown, escape de HTML, medidores y sincronización de tarjetas); pytest `290 passed, 1 skipped`.
- Pendiente: QA manual con mouse (menús de píldoras, desplegar herramientas, Copiar, arrastrar imágenes) y tema claro del sistema. `agent_chat_cards.cpp` tiene ~380 líneas; si crece, separar `ChatToolCard`/`ChatCodeBlock`.

## QA manual: chat visual (modelo, effort, uso, imágenes)

- Hecho: los cuatro puntos de "Correcciones desde QA manual" en ROADMAP.
- Validación: pytest `290 passed, 1 skipped`; CTest `7/7` (el smoke del chat cubre opciones, imágenes, uso/límites y agente no instalado); turnos reales con Codex.
- Pendiente: probar Claude con su CLI instalada (el costo y `contextWindow` dependen de `modelUsage`). Si Claude no lee bien las imágenes por ruta, pasar a `--input-format stream-json`. El orden siguiente está en ROADMAP, punto 6.

## Paso 4: fluidez de Studio — rama `feature/reliable-chat-memory`

- Hecho: detector y `suggested_processes` sin recorrer carpetas de dependencias, más caché de 5 s; canal de lectura separado, caché por comando y refresco automático en Studio.
- Validación: pytest `282 passed, 1 skipped`; CTest `7/7`; build sin warnings. Studio offscreen hace una sola lectura de `bridge status` al arrancar (traza del CLI). No se pudo probar a mano el cambio de vistas ni el refresco: no hay Xvfb y no se abrieron ventanas en el escritorio del usuario.
- Pendiente: QA manual (cambio de vistas instantáneo, que el refresco no mueva selección ni scroll, acciones mientras se lee). Medir el primer frame y la RAM en repos grandes. El import del CLI (~165 ms, typer) puede cargarse en diferido si hace falta.

## Paso 3: chat visual Codex/Claude — rama `feature/reliable-chat-memory`

- Hecho: adapter `integrations/agent_stream.py`, caso de uso `workspace/agent_chat.py`, CLI `agent chat`, `AgentChatModel`/`AgentChatWidget` en Studio y ADR 0009. Toolchain instalado en este Fedora: build en `build/native` con `-DCHXCHX_BUILD_TESTS=ON`.
- Validación: pytest `280 passed, 1 skipped`; compileall; CTest `7/7`; Studio arranca. Turnos reales con Codex por CLI y en el widget (capturas offscreen revisadas). Las salidas largas se recortan y no hay scroll horizontal.
- Pendiente: validar con Claude Code real (no instalado); QA interactivo del panel dentro de Studio (foco, atajos, tema claro); aprobación por acción (`codex app-server` / `--permission-prompt-tool`); mostrar diffs de `file_change`; los chats en curso no cuentan para el RAM Governor; los chats visuales no se reabren al reiniciar Studio.
- Siguiente según ROADMAP: fluidez de Studio (caché del snapshot, lecturas paralelas, refresco automático) y QA en Fedora.

## Fase 2 de mejora: contexto liviano — rama `feature/reliable-chat-memory`

- Hecho: compactación automática y manual, diario de sesiones inyectado en el chat nuevo, y `Estado actual` condensado. En este repo el contexto que lee un chat nuevo bajó de ~95 KB a ~24 KB.
- Validación: compileall OK; pytest `268 passed, 1 skipped`; `context-compact` aplicado aquí con backup.
- Pendiente: comprobar el ciclo completo en una sesión real de Claude/Codex (cerrar, abrir chat nuevo, ver el resumen inyectado). OpenCode no escribe diario porque el diario solo lee transcripts de Codex/Claude. Studio aún no tiene acciones para `context-compact`/`memory-prune`. El resumen es mecánico, sin LLM.
- Siguiente: integración visual de Codex/Claude en Studio (ROADMAP, paso 3).

## Fase 1 de mejora: memoria por chat — rama `feature/reliable-chat-memory`

- Cambios: detector estricto, filtro de contexto inyectado y deduplicación entre chats en `integrations/explicit_memory.py`; carpeta Claude y validación de `cwd` en `integrations/chat_history.py`; comando `workspace memory-prune`.
- Se corrigió `test_layouts.py`: esperaba que OpenCode no capturara memoria, cosa que `a9405bc` ya había cambiado y documentado (ADR 0002).
- Validación: compileall OK; pytest `257 passed, 1 skipped`; `memory-prune --dry-run` sobre este repo no encontró nada que retirar. Falta comprobar en un proyecto real con capturas antiguas que el resultado sea el esperado. En Studio todavía no hay acción para `memory-prune`.
- Siguiente orden acordado con el usuario: Fase 2, contexto liviano (dividir `CURRENT_STATE`/`HANDOFF` en estado vigente corto e historial archivado, y resumen automático al cerrar sesión). Después, integración visual de Codex/Claude en Studio (ver ROADMAP). Luego, la fluidez de Studio (caché del snapshot, lecturas paralelas, refresco automático) y QA en Fedora. Este equipo no tiene cmake, g++ ni qt6-qtbase-devel, así que Studio no se compiló aquí.

## Instalación completa y TUI centrada en Studio

- El workflow produce un `Setup.exe` de usuario para Windows que contiene el instalador PowerShell y conserva el ZIP Qt para el actualizador. Linux Fedora recibe el mismo bundle Qt del SDK que Ubuntu; el script macOS/Linux instala las bibliotecas gráficas nativas con apt/dnf. El verificador exige QtCore, QtWidgets, Core5Compat y `libqxcb.so` en ambos TGZ.
- La TUI abre ChxChx Studio en segundo plano. `studio open` queda como comando explícito y Studio es el editor predeterminado de proyectos nuevos; Sublime queda como adapter opcional para configuraciones existentes.
- La decisión durable quedó en `.ai/adr/0008-studio-first-editor.md`.
- Validación local: compileall de `src`/`scripts`, sintaxis Bash y `git diff --check` pasan. No se ejecutaron pruebas. Falta el build CI de NSIS/paquetes y smoke de instalación en máquinas limpias. No hay PowerShell/NSIS disponibles en este entorno. La publicación del preview continúa aplazada, así que aún no se prueba una descarga desde el release.

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
