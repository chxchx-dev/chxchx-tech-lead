# Handoff

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

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

## README: uso de Studio y TUI

- README abre con una elección entre Studio, TUI y CLI y comandos de primer uso para cada ruta. La guía de Studio cubre abrir el proyecto por ruta, `Preparar y entrar`, edición, terminal, agentes y memoria; la guía TUI cubre `init`, trust, áreas, atajos y regreso de Zellij.
- Se acortó el banner inicial y se organizaron las secciones largas en pasos, tablas y temas. Se conservaron las referencias técnicas para desarrollo y operación.
- Verificación: `git diff --check`. No se ejecutó una suite: el cambio es documentación.

## OpenCode y memoria de agentes

- OpenCode ya estaba disponible para añadir desde Studio; se documentó la conexión MCP fácil en `Configuración → Configurar MCP · OpenCode` (Basic Memory y Serena, CLI oficial, alcance de proyecto).
- Se agregó la captura de memoria explícita con `opencode session list` y `opencode export --sanitize`. Se valida la raíz exacta del proyecto, se limita a diez sesiones recientes y se reutiliza el archivo `.ai/memory/PROJECT_MEMORY.md` que comparten los agentes. Las sesiones nuevas recuperan esas notas. No se lee la base interna de OpenCode.
- Actualizados README, ADR 0002, EDITOR-PARITY y CURRENT_STATE. No se ejecutaron pruebas; pendiente smoke manual de OpenCode y Ollama.
- Se conservaron los cambios locales existentes del actualizador de Studio y los Mods añadidos previamente.

## Instalador y actualizador de Studio

- Los instaladores Bash/PowerShell se pueden repetir para instalar el preview más reciente; `--update`/`-Update` requiere un launcher administrado y `--dry-run`/`-DryRun` revisa checksums y conflictos antes de escribir. Los marcadores guardan el SHA completo, y se migra el marcador vacío de versiones Bash anteriores.
- En Studio, **Ayuda → Buscar actualizaciones…** consulta el manifiesto, muestra la versión disponible y solicita confirmación. App y bridge CLI se preparan en una carpeta versionada; se valida el nuevo CLI antes de cambiar el launcher y se mantiene la versión anterior para rollback. El ZIP de Windows se normaliza si CPack incluye una carpeta raíz adicional.
- `README.md`, ADR 0005, `CURRENT_STATE` y este handoff documentan el flujo. Aún falta ejecutar una instalación/actualización interactiva en máquinas limpias de Windows, macOS, Ubuntu y Fedora; la publicación del preview sigue aplazada.
- Verificación local: `python -m compileall -q src`, `bash -n scripts/install-studio.sh scripts/install-studio-fedora.sh`, `cmake --build build/native --parallel 2` y `git diff --check` pasan. No se ejecutaron suites de tests ni instalaciones reales; `pwsh` no está disponible para parsear el script PowerShell localmente (CI contiene ese chequeo en Windows).

## Mods de Claude Code

- El catálogo pasó a nueve: se añadieron `chxchx-git-workflow`, `chxchx-safe-refactor`, `chxchx-tech-docs` y `chxchx-performance`. Son guías Markdown instaladas por proyecto; rendimiento se recomienda según stack/lenguaje detectado.
- Se actualizaron README, ADR 0004 y CURRENT_STATE. No se ejecutaron verificaciones en esta tarea.
- El checkout ya contenía cambios no relacionados del actualizador integrado de Studio; se conservaron.

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
