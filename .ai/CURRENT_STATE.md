# Current State

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Instaladores y canales de release (ADR 0013)

- `scripts/install.sh` (Linux/macOS) e `scripts/install.ps1` (Windows): canal stable/dev, `--version`, `--update`, `--cli-only`, `--uninstall`, `--dry-run`, verificación SHA-256 con el manifiesto, nada ajeno se sobrescribe. `scripts/release_assets.py` genera manifiesto + `SHA256SUMS`. Wrappers: `install-studio.*`, `install-studio-fedora.sh`, `bootstrap.*`. `windows-studio-setup.nsi` empaqueta `install.ps1` (`-DCHANNEL`).
- CMake lee la versión de `pyproject.toml`; Studio responde `--version`/`--help` antes de Qt; iconos en `native/packaging/` (png/ico/icns, `.rc` en Windows). Updater Python (`studio_channels.py`, `studio_update_launcher.py`) sigue el canal instalado; corregido que `parse_manifest` rechazaba `schema_version`.
- CI: `stage-release` → `installer-smoke` (ubuntu, macOS x2, Windows, servidor localhost + `CHXCHX_TEST_BASE`) → `publish-studio-preview` / `publish-studio-stable` (tag `v*`, con `check-release-tag`). Firma condicionada a secretos (`WINDOWS_CERT_*`, `MACOS_*`, `APPLE_*`).
- Verificado en Linux: `tests/installers` (37) y `tests/integrations/test_studio_channels.py` (8), build local de Studio con `--version`. **No verificado**: `install.ps1`, NSIS, flujo DMG, `installer-smoke` y publicación real, firma (sin pwsh/macOS/certificados aquí).

## Asistente de Git: modelo ligero por defecto

- `git_assistant_models` (puro): `parseAgentOptions`, `looksLight` (por nombre o por tamaño ≤14b), `lowEffort`, `chooseModel` (automático / `__agent__` / un modelo concreto), `describeChoice`. El controlador lee la lista de modelos del agente (`bridge agent-options`) una vez y pasa `--model`/`--effort` a `agent chat --lean`; la tarjeta tiene el submenú Modelo (se reconstruye solo si algo cambió, para no cerrarse mientras se usa).
- Pruebas: elección pura en `git_smoke` (nombres, tamaños, esfuerzo, Haiku sin esfuerzo, modelo desconocido) y los argumentos reales de cada consulta en `git_ui_smoke`; 2 mutaciones detectadas; 12 ejecuciones del UI sin fallos.
- Sin probar con los modelos reales de cada CLI (¿Claude acepta `--effort low` con un modelo concreto?, ¿qué modelos ligeros publica Codex?).

## Asistente de Git y rediseño del panel

- Python: `agent chat --lean` (sin contexto/memoria/diario, solo lectura, sin sesión: `--ephemeral` Codex, `--no-session-persistence` Claude, borrado de la sesión en OpenCode) y `--prompt-file`; `ChatTurnPlan.lean`, `load_prompt`. 378 pruebas pasan.
- Nativo (`native/src/git/`): `git_assistant_prompts` (redacción de secretos y archivos sensibles, prompts, bloques de conflicto, limpieza del mensaje), `git_guide` (siguiente paso por reglas), `git_assistant` (ejecuta `agent chat --lean` con el prompt en un archivo temporal, cancelable), `git_assistant_controller` (agente, consentimiento, contexto, estado fresco de la fusión), `git_assistant_bar` (tarjeta con tono + selector de agente), `git_advice_dialog`. Botón Sugerir, "Pedir consejo" en la tarjeta y en los avisos de error.
- Diseño: iconos de Studio en pestañas, rama, sincronización, secciones, acciones de fila, diff y ramas (nuevos: list-add/remove, go-up/down, git-commit/merge/pull/push/stash, check-circle, studio-assistant, studio-advice); botón de commit primario con el acento; cajas redondeadas; estado vacío con icono; la caja del mensaje crece.
- Pruebas: la lógica pura en `git_smoke` (redacción, prompts, guía, limpieza, parser de salida) y los flujos con un `chxchx-tech` falso en `git_ui_smoke` (sin agente, sugerir, consejo, error, cancelar, cambiar de agente, conflictos). 11 tests nativos pasan; 10+15 ejecuciones seguidas del UI sin fallos; 4 mutaciones de privacidad (archivo sensible enviado, secretos sin enmascarar, diff sin enmascarar, sin `--lean`) detectadas.
- Bugs encontrados al probar: arranque con fallo de segmentación (el asistente se configuraba antes de crear el cliente del bridge) y una carrera (el consejo de conflictos no sabía si la fusión estaba a medias): corregidos.
- Sin probar con Codex/Claude/OpenCode reales (flags efímeros, borrado de sesión) ni con el consentimiento en un escritorio.

## Visor de Git en Studio (fase 2: enviar/recibir, stash y explorador)

- Motor: `git_repository_remote.cpp` (`fetch`, `pull` ff-only o merge, `push`/publicar, `mergeInProgress`/`abortMerge`, stash), `GitProblem` (credenciales, rechazado, ramas separadas, sin upstream, sin remoto, identidad, cancelado), cancelación de operaciones lentas, procesos en sesión nueva sin terminal. `git_explorer` (estado por ruta) y `GitTreeDelegate` (color, letra y punto en carpetas). Panel: botón de sincronización con acción inteligente, confirmación de envío (`git/confirmPush`), diálogo al separarse las ramas y sección Apartados.
- Pruebas: `git_smoke` crece con dos clones y un remoto bare (publicar, recibir, enviar, divergencia, rechazo, conflicto con abandono, sin remoto, cancelación, stash); `git_ui_smoke` cubre los flujos del panel. 11 tests nativos pasan; 20 ejecuciones seguidas sin fallos; 5 mutaciones (publicar sin `-u`, pull sin ff-only, push forzado, cancelar sin efecto, rechazo sin reconocer) detectadas.
- Gancho de QA: `CHXCHX_STUDIO_SCREENSHOT_TOOL=git:files` muestra el explorador. Capturas: `studio-git-sync.png`, `studio-git-explorer.png`.
- Sin probar contra un remoto real con autenticación (SSH/HTTPS), ni en Windows o macOS. Decisión: sin rebase ni envío forzado.

## Visor de Git en Studio (fase 1)

- `native/src/git/`: `git_types`, `git_parsers` (estado v2, ramas, log + carriles del grafo, diff unificado en filas lado a lado, name-status), `git_repository` (cola serializada asíncrona; stage/unstage/discard con copia en `.git/chxchx-backups`, commit por stdin, ramas, log, diffs), vistas `GitChangesView` (+`GitChangeDelegate` con acciones al pasar el cursor), `GitHistoryView` (+`GitGraphDelegate`), `GitBranchesView`, `GitDiffView` y `GitPanel`. Integración en `main_window_git.cpp`: pestaña Archivos|Git, `openGitDiff`, `Ctrl+Shift+G`.
- Pruebas nativas: `native-git-smoke` (parsers + repo real) y `native-git-ui-smoke` (panel completo, diff, cierre con trabajo en vuelo); 11 tests nativos pasan; 24 ejecuciones seguidas sin fallos. Se corrigieron dos fallos de ciclo de vida (destructor del repositorio y callback de una pestaña de diff cerrada) y se comprobó con mutaciones que las pruebas los detectan. Sin ASan/valgrind en esta máquina.
- Gancho de QA ampliado: `CHXCHX_STUDIO_SCREENSHOT_TOOL=git:changes|history|branches` y `git-diff:ruta|fuente`. Capturas del README en `assets/screenshots/studio-git-*.png` (repo de ejemplo).
- Un commit de merge muestra lo que trajo respecto al primer padre (`-m --first-parent`).
- No validado en Windows ni macOS; falta push/pull/fetch, stash, blame, conflictos con editor y estado de Git en el explorador (ver ADR 0011).

## Selector de agentes con OpenCode, README y capturas

- Inicio: el selector añade la entrada `＋ Añadir OpenCode` (rol `AddAgentRole`) si el proyecto no tiene un agente `opencode`; el botón pasa a "Añadir OpenCode" y reutiliza `MainWindow::addAgentToProject` (dry-run + confirmación, igual que Agentes → Añadir agente). No se modifica ningún proyecto en silencio: cambiar la config invalida el trust.
- Gancho de QA `CHXCHX_STUDIO_SCREENSHOT_TOOL` (`local-models`, `add-agent`, `chat:AGENTE|mensaje`) vía `MainWindow::showTool`.
- README: sección "OpenCode y modelos locales" breve con 3 capturas (`assets/screenshots/`, datos de ejemplo, sin rutas ni metadatos), comandos `local-models` en la lista, y el chat visual documentado para los tres agentes.
- 373 pruebas de Python y 9 tests nativos pasan. La entrada del selector se verificó con captura offscreen; no se probó el flujo completo de añadir (trust incluido) en un escritorio.

## Pruebas en capas (E2E con falsos y live)

- `tests/fakes/` (`FakeOllama` HTTP real y `fake_clis` con `ollama`/`opencode` como procesos), `tests/conftest.py` (`isolated_home`, `fake_ollama`, `fake_bin`), `tests/e2e/` (adapter por HTTP, `setup`/`remove`/`status` por CLI, chat de OpenCode con stdin/permisos/reanudación/diario) y `tests/live/` (10 pruebas opt-in; `addopts = "-m 'not live'"`). Guía en `tests/README.md`.
- 373 pruebas pasan, 10 live deseleccionadas (se saltan limpias sin las herramientas). Se hicieron 8 mutaciones del código y todas las detectaron las pruebas.
- Las pruebas live de OpenCode se ejecutaron contra el falso solo para validar sus helpers (pasa la de stdin; las de lectura de archivos y memoria necesitan un modelo real).
- Posible bug encontrado y corregido: `Popen(["opencode"])` no resuelve `opencode.cmd` en Windows; ahora `resolve_executable` lanza por ruta completa (agent_chat, `ollama create`, `opencode models`). **No verificado en Windows.**

## Modelos locales: quitar, cargados y acceso desde Inicio

- `local_models_remove.py` + `plan_remove_ollama_models` + `ollama.delete_model`/`LoadedModel`; CLI `local-models remove`; payload con `ollama.loaded` y `models[].loaded`. Studio: botón Quitar (confirmación), "En memoria" en la tabla y el encabezado, y botón "Modelos locales" en Inicio (`m_homeLocalModelsButton`).
- 348 pruebas de Python y los 9 tests nativos pasan (la prueba nativa cubre cargados, Quitar y su resumen). El botón de Inicio solo se comprobó por compilación: la captura offscreen no mostró esa pestaña.
- Sigue sin validarse con Ollama/OpenCode reales; el área Agentes no tiene acceso (su barra admite 4 acciones).

## Pestaña "Modelos locales" de Studio

- `native/src/integrations/local_models_{model,widget,widget_view,widget_setup,widget_style}` + `LocalModelsWidget`: lee `bridge local-models`, ejecuta `local-models setup --json` y muestra aviso de Ollama, tabla con encaje, vista previa, confirmación de descarga, progreso y log. Se abre con `Herramientas → Modelos locales…` (`MainWindow::openLocalModels`, reutiliza la pestaña); cerrar con descarga en curso pide confirmación.
- `native-local-models-smoke` cubre parseo, estados (sin Ollama / detenido / listo), eventos y un refresh por CLI falso; `CHXCHX_LOCAL_MODELS_SCREENSHOT` captura la pestaña. Studio completo compila y los tres smokes pasan; 336 pruebas de Python pasan.
- Se corrigió un desajuste de contrato: `preview` envía `path` y `done` `config_path`; el parser acepta ambos.
- **No validado con Ollama ni OpenCode reales** (no instalados) ni con la ventana de Studio en un escritorio.

## Modelos locales: Ollama + OpenCode (backend)

- `integrations/`: `local_hardware`, `local_model_catalog`, `local_model_fit`, `ollama`, `opencode_local_config`, `local_models_setup`; `workspace/bridge_local_models.py`; CLI `local-models status|setup` (`commands/local_models.py`) y `bridge local-models`. Diseño en ADR 0010.
- Decisiones del usuario: backend primero y pestaña después; config en el `opencode.json` del proyecto; Studio detecta Ollama y descarga modelos con confirmación, pero no lo instala.
- `core/backup.py::backup_file` crea un área aparte; no mezclar con `backup_project` (su restore borra archivos gestionados ausentes).
- 336 pruebas pasan. `local-models status` corrió en esta máquina (21 GB RAM, Radeon 2 GB): recomienda solo `qwen2.5-coder:7b`. **No validado con Ollama real** (no instalado).

## OpenCode en el chat visual y el diario de sesiones

- `integrations/agent_stream_opencode.py` normaliza `opencode run --format json` al contrato `chxchx.agent-event` v1; `agent_stream.py` añade el proveedor `OpenCode`, `turn_command` (`--agent plan|build`, `--session`, `--model`, `--variant`, `--file`) y `turn_environment` (en solo lectura, `OPENCODE_PERMISSION` niega `edit` y `bash`).
- `agent_catalog.py` ofrece modelos de `opencode models` y el modelo por defecto de `opencode.json`; `chat_history_opencode.py` lee la sesión más reciente del proyecto para el diario. `agent_chat.py` pasa env y el ejecutable de OpenCode.
- Studio: `supportsProgram` acepta `opencode`, avatar `◇` y textos actualizados; `native-agent-chat-smoke` compila y pasa. El agente se añade con `chxchx-tech agent add opencode`.
- 302 pruebas pasan y `compileall` está limpio. **No validado con la CLI real**: OpenCode no estaba instalado y el esquema de eventos sale de parsers de la comunidad. Ver ADR 0009.

## README y ramas

- `main` se integró en `dev` (`cd86d46`) y los 10 commits de `feature/reliable-chat-memory` también quedaron integrados (`9ceb081`): memoria por chat, diario/compactación de contexto, chat visual y mejoras de Studio.
- README conserva sus instrucciones y ahora abre con el arte ASCII solicitado, badges y enlaces rápidos a Studio, TUI y CLI.
- `dev` se publicó en `origin` con el commit `5eae006`; los 10 commits de `feature/reliable-chat-memory` están integrados y las referencias local y remota de esa rama se eliminaron.

## Responsivo y pendientes menores del rediseño

- `ResponsiveGrid` (`responsive_grid.cpp`) reparte los widgets visibles en 1–4 columnas según el ancho y se reordena al mostrar u ocultar alguno. Se usa en "Acciones rápidas" de Inicio, en la barra de acciones de área y en las sugerencias del chat.
- Las acciones de área (selector y 4 botones) pasaron del dock "Secciones y acciones" a `#areaActionBar`, sobre las páginas, dentro de `m_sectionContainer`, que ahora es la pestaña "Inicio". La lista de secciones mantiene su alto al cambiar de área. Las páginas viven en un `QScrollArea`, así que la ventana puede tener cualquier alto. Los docks tienen ancho mínimo (200/210) y uno inicial razonable.
- El editor parte líneas conservando la sangría y con marca de continuación. Markdown colorea títulos completos (`lexer.markdown.header.eolfill`). `StudioFileIconProvider` pinta iconos por tipo de archivo con colores del tema. La terminal elige una paleta ANSI clara u oscura según la luminosidad del fondo.
- Chat en paneles estrechos: píldoras en modo compacto (sin "· por defecto"), sugerencias en una columna y texto de ayuda oculto. Se validó a 460 px.
- Nuevo CTest `native-layout-smoke` (columnas, reflujo al ocultar, tipos de archivo). La captura acepta `CHXCHX_STUDIO_SCREENSHOT_SIZE` y `CHXCHX_STUDIO_SCREENSHOT_AREA`.

## Rediseño visual y temas

- `studio_theme.{hpp,cpp}` + `studio_themes.cpp`: `StudioTheme` con tokens de interfaz, del editor y de sintaxis. Seis temas originales (medianoche, grafito, aurora, bosque, ambar, claro). `ThemeManager` aplica paleta y hoja de estilos, guarda `appearance/theme` en QSettings y emite `themeChanged`. `studio_stylesheet.cpp` es una plantilla con `@tokens@` y `@img:…@`, que apuntan a PNGs generados por tema.
- `studio_icons.cpp`: ~45 iconos de línea dibujados con QPainter mediante un `QIconEngine` que pinta con el color del tema activo (muted/hover/accent/disabled). `studioIcon()` conserva su firma y hace fallback al tema del sistema.
- `editor_theme.cpp`: roles semánticos por lexer con las constantes de `SciLexer.h` y listas de palabras clave. Corrige el mapeo anterior (mismos números de estilo para todos los lenguajes) y que JS/TS/HTML no se coloreaban porque esos lexers no existen en Lexilla: ahora usan `cpp` y `hypertext`. Añade Rust, Go, Java, C#, Kotlin y PowerShell. Incluye fuente mono preferida, resaltado de paréntesis y marcadores de plegado según el tema.
- `appearance_dialog.cpp` (tarjetas con vista previa), submenú Ver → Tema, `Ctrl+K Ctrl+T`, entradas `Tema: …` en la paleta y barra de estado (Ln/Col, lenguaje, tema). La terminal toma sus colores por defecto de la paleta. El chat expande su hoja de estilos desde el tema.
- `chxchx-studio PROYECTO archivo…` abre archivos; `CHXCHX_STUDIO_SCREENSHOT` captura la ventana para QA.
- Claude Code 2.1.316 ya está instalado aquí y quedó validado en el chat: imagen por ruta, `--model`/`--effort`, costo, `contextWindow` y `--resume`. El uso de Claude suma los tokens en caché a la entrada.

## Chat visual con estilo de IDE

- El transcript dejó de ser HTML en `QTextBrowser`. `AgentChatView` (QScrollArea) sincroniza `ChatEntryCard`s nativas por revisión: burbuja del usuario a la derecha con miniaturas, respuestas con avatar por proveedor (Codex verde `</>`, Claude naranja `✳`), `ChatCodeBlock` con lenguaje y Copiar, `ChatToolCard` desplegable con spinner y borde según estado, razonamiento plegable y errores con el comando copiable.
- Bienvenida con sugerencias, indicador animado de "trabajando", cabecera con identidad y `ChatUsageBar` (barras de contexto, 5 h y semana, más el gasto). El composer es redondeado con píldoras `ChatPillPicker` (modelo, effort, permisos) y botón circular enviar/detener.
- El tema de Studio se movió a `studio_theme.{hpp,cpp}` (`applyStudioTheme`, `StudioColors`). El chat usa `agent_chat_style.hpp`, limitado por object names.
- El modelo ya no genera HTML (`Entry::revision`). El markdown pasa por `agent_chat_markdown` (fences separados y HTML crudo desactivado).

## Chat visual: modelo, effort, uso e imágenes (QA manual)

- Con un agente no instalado, `agent chat` y el panel avisan con el comando de instalación; ya no aparece `[Errno 2]`. El panel desactiva el envío.
- `integrations/agent_catalog.py`: opciones por agente (catálogo local de Codex, alias de Claude), disponibilidad, instrucciones de instalación y `codex_limits` (contexto, límites de 5 h/semanal y plan, leídos del rollout). Endpoint `bridge agent-options`.
- Studio: `AgentChatSettings` (modelo/effort según cada modelo), `AgentChatComposer` (📎, Ctrl+V, arrastrar y soltar) y barra de uso en `AgentChatWidget`, alimentada por `limits`/`usage`.
- Validado con Codex real: modelo, effort, imagen en chat nuevo y en `resume`, evento `limits`. Capturas offscreen revisadas. Claude sigue sin validar en vivo.

## Fluidez de Studio

- `bridge status` bajó de ~0,70 s a ~0,37 s. El detector recorre el árbol una sola vez sin entrar en `IGNORED_DIRS` (antes, tres `rglob("*.csproj")` entraban en `.venv`/`node_modules`/`build`) y guarda el resultado 5 s por proceso. `suggested_processes` poda las carpetas ignoradas durante el recorrido.
- Studio usa un `BridgeClient` solo para lecturas (`main_window_refresh.cpp`), así que las consultas ya no bloquean acciones. Hay caché por comando: las vistas que comparten snapshot se pintan al instante y se refrescan en segundo plano si el dato tiene más de 3 s. Cada acción y cada cambio de proyecto invalidan la caché; un contador de generación descarta lecturas que empezaron antes de la acción.
- Refresco automático cada 10 s de Inicio/Proyecto/Agentes/Procesos/Recursos, solo con la ventana activa y sin diálogos ni desplegables abiertos.
- `finishCommand` delega la interpretación de esquemas en `applyBridgePayload(payload, area)`.

## Chat visual con Codex y Claude (ADR 0009)

- `chxchx-tech agent chat` ejecuta un turno estructurado (`codex exec --json` / `claude -p --output-format stream-json`) y emite JSONL `chxchx.agent-event` v1. Requiere trust, envía el prompt por stdin, se cancela por stdin y escribe memoria y diario al terminar.
- Studio: botón **Chat visual** en Inicio, `Herramientas → Chat visual con agente` y paleta. Pasa por preflight de trust/RAM, abre una pestaña con mensajes en markdown, tarjetas de herramientas, permisos y Detener/Nuevo chat.
- `workspace/chat_context.py` concentra la preparación del contexto y la captura posterior, compartidas por la pestaña PTY y el chat visual.
- Validado con Codex 0.166 real en Fedora: dos turnos con reanudación, y el agente usó el resumen del diario. Claude no está instalado aquí; se implementó con su formato documentado.

## Contexto liviano y diario de sesiones (Fase 2)

- `workspace/context_compaction.py` mueve las secciones antiguas de `CURRENT_STATE`/`HANDOFF` a `.ai/history/` (se conservan 6 recientes, ~12.000 caracteres). Las secciones de `init` y los bloques administrados no se mueven. Hay backup vía `backup_project`. Se ejecuta antes de cada chat nuevo salvo con `auto_compact_context = false`, y a mano con `workspace context-compact`.
- `workspace/session_journal.py` resume al salir cada sesión de Codex/Claude en `.ai/history/SESSIONS.md` (local, ignorado por Git) y adjunta las 2 más recientes al prompt de chat nuevo. El prompt pide no leer `.ai/history/` salvo necesidad y no repetir historial.
- En este repo, "Estado actual" se condensó a mano; la versión extensa y los historiales están versionados en `.ai/history/` (`git add -f`).

## Memoria por chat confiable (Fase 1)

- La captura de memoria explícita solo acepta directivas al inicio de frase o tras "y"/"por favor" (ES/EN). Antes cualquier mención de "memoria" o "guarda" se guardaba e inyectaba en cada chat nuevo como petición literal.
- Se ignoran los bloques que el agente inyecta en el turno del usuario (`<system-reminder>`, selección del IDE, salida de comandos). Una misma petición repetida en otro chat se guarda una sola vez.
- La búsqueda de transcripts Claude usa el nombre de carpeta de Claude Code (todo carácter no alfanumérico → `-`, sufijo hash en rutas largas) y conserva el formato anterior. Como el nombre es ambiguo, cada transcript debe tener su `cwd` igual a la raíz del proyecto.
- `chxchx-tech workspace memory-prune PATH [--dry-run]` reevalúa las capturas automáticas de `PROJECT_MEMORY.md`, conserva las notas manuales y deja un backup fuera del repo.

## Instalación completa y TUI centrada en Studio

- Windows genera `chxchx-studio-windows-x86_64-setup.exe` con NSIS. El ejecutable incluye el instalador PowerShell, que descarga y valida el ZIP Qt y el bridge CLI; volver a abrirlo actualiza. El ZIP se mantiene para el actualizador integrado.
- Ubuntu y Fedora ahora requieren los mismos componentes Qt y el plugin xcb dentro del TGZ; el instalador común macOS/Linux añade con apt/dnf las bibliotecas gráficas del sistema. macOS conserva la app con frameworks desplegados en el DMG. El workflow comprueba los runtimes Qt para ambos paquetes Linux.
- La TUI abre ChxChx Studio en segundo plano y ya no ofrece preparar un proyecto Sublime. `chxchx-tech studio open` abre Studio; proyectos nuevos lo configuran como editor. Sublime sigue disponible si un proyecto lo configura explícitamente.
- Validación local: `python -m compileall -q src scripts`, `bash -n scripts/install-studio.sh scripts/install-studio-fedora.sh` y `git diff --check` pasan. No se ejecutó la suite. Este entorno no tiene PowerShell ni NSIS, así que el nuevo `Setup.exe` y el workflow todavía requieren build CI. No se probó instalación en equipos limpios; la publicación del preview sigue aplazada y el enlace de descarga aún no estará disponible hasta publicarla.

## Estado actual

- Producto: control plane local con CLI/TUI en Python y ChxChx Studio en C++20/Qt 6 Widgets con Scintilla/Lexilla. Studio llama al CLI mediante el bridge JSON v1 (`workspace/bridge_contracts.py`); Python sigue siendo la autoridad. La rama de integración es `dev`.
- Studio cubre todas las áreas de la TUI (inventario en `.ai/EDITOR-PARITY.md`). Toda escritura pasa por `--dry-run`, preview y confirmación. Inicio ofrece `Preparar y entrar`. Incluye terminal PTY/ConPTY + libvterm (scrollback de 5000 líneas), pestañas de agentes Codex/Claude/OpenCode (+Ollama), búsqueda de rutas y contenido, archivos recientes y grupos divididos, diff Git por archivo, símbolos básicos, atajos configurables y paleta `Ctrl+P`.
- Catálogo: 21 skills, 13 Tech Packs y 9 Mods de Claude Code en `.claude/skills`. RAM Governor: `max_agents` 2, aviso RAM 70 %, crítico 85 % y aviso swap 40 %; trust local obligatorio antes de ejecutar procesos.
- CI/CD de proyectos del usuario: CI opt-in para Python/Node/CMake y CD opt-in a VPS con Docker Compose (ADR 0007), ambos en CLI/TUI/Studio.
- Distribución: CPack genera ZIP/DMG/TGZ y un `Setup.exe` NSIS. El workflow está preparado para el release rodante `studio-dev-preview`; el instalador oficial respondió 404 al comprobarlo. El run #87 de CI validó los paquetes en todos los runners. Nunca se ha probado la instalación en máquinas limpias.
- Memoria: `.ai/memory/PROJECT_MEMORY.md` guarda las peticiones explícitas, y Basic Memory se usa limitado al `memory_project` del proyecto. Al cerrar cada sesión de Codex/Claude se escribe un resumen en `.ai/history/SESSIONS.md`, que se inyecta en el siguiente chat nuevo. `CURRENT_STATE`/`HANDOFF` se compactan solos hacia `.ai/history/`.
- Lo que bloquea el uso diario: QA interactivo de Studio en Fedora con un proyecto real. El orden de trabajo está en `.ai/ROADMAP.md` (plan de noviembre 2026).
- Equipo actual: Fedora 44 con CMake 4.3, GCC 16, Qt 6.11 y Ninja. Studio se compiló e instaló desde `dev` (`5eae006`) en `~/.local/opt/chxchx-studio/dev-5eae006`; el launcher está en `~/.local/bin/chxchx-studio-dev` y hay entrada de menú. El launcher usa el bridge CLI del `.venv` de este checkout y los plugins Qt del sistema. El estado anterior completo está en `.ai/history/CURRENT_STATE.md`.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.
