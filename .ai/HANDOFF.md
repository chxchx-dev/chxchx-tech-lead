# Handoff

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## README y merge de ramas

- Se integró `main` en `dev` (`cd86d46`) y luego `feature/reliable-chat-memory` (`9ceb081`); los 10 commits de esa rama quedan conservados en `dev`.
- Se actualizó la cabecera del README con el arte ASCII solicitado, badges de CI/Python/Qt/plataformas y navegación rápida. Se conservaron las guías, comandos y detalles que ya estaban documentados.
- No se hizo push. `dev` está 15 commits por delante de `origin/dev`; `.vscode/` sin seguimiento se preservó. Se eliminó la rama local ya integrada; la referencia remota seguirá como respaldo hasta que se publique `dev`.
- Verificación: `git diff --check` sin errores. No se ejecutaron tests en esta ronda.
- Basic Memory no está disponible en las herramientas de esta sesión; el checkpoint quedó en `.ai/CURRENT_STATE.md` y este archivo.

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
