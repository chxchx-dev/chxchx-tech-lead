# Historial de CURRENT_STATE

Secciones archivadas automáticamente, de la más reciente a la más antigua.

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

## README: guía de inicio para Studio y TUI

- El comienzo del README ahora ayuda a elegir Studio, la TUI o la CLI y ofrece pasos concretos para lanzar cada interfaz sobre un proyecto.
- La guía de Studio explica instalación/launcher, **Preparar y entrar**, edición, búsqueda, terminal, agentes y memoria. La TUI documenta preparación con `init`, trust, áreas principales, teclas y salida de Zellij.
- Se sustituyó el banner ASCII extenso y los bloques de uso de Studio/TUI se dividieron por recorrido y tema. Se mantienen las referencias técnicas y el catálogo de capacidades.

## OpenCode y memoria compartida entre agentes

- Studio ya permite añadir OpenCode u OpenCode con Ollama desde Agentes y conectar MCP de Basic Memory/Serena desde Configuración, con preview, confirmación y alcance local al proyecto.
- El wrapper ahora captura solicitudes explícitas desde sesiones OpenCode con `opencode session list` y `opencode export --sanitize`, valida que la sesión pertenezca a la ruta exacta del proyecto y comparte `.ai/memory/PROJECT_MEMORY.md` con Codex y Claude. Al iniciar chat nuevo, recupera y adjunta las solicitudes locales sin duplicarlas.
- No se lee la base de datos interna de OpenCode. La captura está acotada a las diez sesiones recientes y a cinco segundos por comando. La vista Chats permanece enfocada en Codex/Claude.
- No se ejecutaron pruebas en esta tarea; falta smoke manual con OpenCode normal y OpenCode con Ollama.

## Catálogo de Mods para Claude Code

- Se amplió de cinco a nueve guías locales con flujo Git, refactorización segura, documentación técnica y rendimiento basado en mediciones. Siguen siendo instrucciones instaladas por proyecto, sin hooks, ejecutables ni cambios globales.
- La recomendación de rendimiento depende de stacks/lenguajes detectados; las otras tres guías son de flujo general. README y ADR 0004 describen el catálogo actualizado.
- La instalación y actualización del preview de Studio se documenta en la sección siguiente.

## Studio: funcional; validación final para uso diario pendiente

- Studio tiene la funcionalidad principal implementada y los instaladores nativos pasaron CI: run #87 terminó con los 15 jobs aplicables en verde. La rama feature no publicó `studio-dev-preview`, de acuerdo con la decisión de aplazar esa publicación.
- Aún no se debe marcar como validado para uso diario. Falta recorrer en Fedora, con un proyecto confiable, Init y trust, skills/sync, edición y guardado, terminal, sesión de agente con contexto, preflight de RAM y cierre limpio. Tampoco hay evidencia de instalación interactiva en una segunda máquina Fedora.
- El menú **Ayuda → Buscar actualizaciones…** consulta el release y pide confirmación antes de aplicar. El actualizador valida los SHA-256, instala app y bridge CLI en una carpeta versionada, valida que el CLI nuevo arranque y cambia el launcher al final; conserva la versión anterior para rollback. El descubrimiento de instalación admite el marcador SHA completo y el marcador vacío de instaladores antiguos.
- Los instaladores Bash y PowerShell se pueden volver a ejecutar para actualizar; `--update`/`-Update` exige una instalación previa y `--dry-run`/`-DryRun` valida descargas y conflictos antes de escribir. Windows normaliza ZIP con raíz anidada. No se probó una instalación interactiva en equipos limpios; la publicación del preview continúa aplazada y el chequeo en Studio requiere que haya un release publicado.
- Verificación local: `python -m compileall -q src`, `bash -n` de instaladores, build nativo Qt y `git diff --check` pasan. No se ejecutó pytest ni una instalación; `pwsh` no está disponible localmente.

## Estado actual (versión extensa archivada el 2026-11-01)

- Inicio de Studio ofrece `Preparar y entrar`: vista previa y confirmación de Init, skills recomendadas, skills de Tech Packs detectados y Mods de Claude aplicables, sincronización del contexto, trust local y apertura de un chat nuevo con el agente elegido. Los Mods se incluyen solo si el proyecto ya tiene contexto/configuración Claude y nunca modifican ajustes globales.
- El catálogo contiene nueve Mods instalables y retirables en `.claude/skills`: asesor de modelo, revisión previa a integrar, depuración con evidencia, planificación, guía C++/Qt, flujo Git, refactorización segura, documentación técnica y rendimiento. Studio tiene vista Mods con recomendaciones, estado, aplicación individual/por detección y retiro; el CLI expone `mod list/apply/remove`. El asesor muestra `/model <alias>` para que el usuario confirme el cambio dentro de Claude Code.
- CI prepara un release rodante `studio-dev-preview` para Ubuntu/Fedora x86_64, Windows x86_64 y macOS Intel/Apple Silicon, más el wheel del bridge CLI con su nombre versionado válido. El manifiesto SHA-256 y los instaladores de Linux/macOS/Windows permiten instalar sin clonar. La publicación del preview continúa aplazada.
- Validación de este bloque: `python -m compileall -q src tests scripts`; pytest (`217 passed, 1 skipped`); `cmake --build build/native --parallel 2`; CTest (`6/6`); CLI JSON de `mod list`; `projects prepare . --dry-run --trust`; sintaxis Bash, ayuda de ambos instaladores, nombre de wheel versionado con `packaging` y `git diff --check` pasan. No se pudo descargar el release con `--dry-run` ni construir el wheel localmente porque la sesión no resuelve PyPI/raw.githubusercontent.com; `pwsh` tampoco está instalado. El workflow se validó sintácticamente como YAML, pero necesita CI tras el push para verificar los assets por sistema.
- Studio añade OpenCode u OpenCode con Ollama desde Agentes, con `--dry-run`, confirmación y backup idempotente de `.ai/chxchx-tech.toml`. Configuración ofrece `Configurar MCP · OpenCode` para conectar Basic Memory y Serena mediante la CLI oficial. El wrapper usa `opencode session list` y `opencode export --sanitize` para capturar solicitudes explícitas y recuperar memoria del mismo proyecto. Ollama hace su selección guiada de modelo al iniciar `ollama launch opencode`.
- Validación de esta integración: `python -m compileall -q src tests scripts`, pytest (`210 passed, 1 skipped`), `git diff --check`, build Qt Linux y CTest (`6/6`). No se cambió la configuración local del proyecto durante el desarrollo.
- Catálogo ampliado a 21 skills y 13 Tech Packs: añade ingeniería C++, desarrollo Qt 6, builds CMake/empaquetado y software nativo multiplataforma. El detector reconoce fuentes C++ y proyectos Qt para recomendar estas guías y packs automáticamente.
- Editor y navegación visualmente refinados: colores Scintilla en el orden RGB correcto; pestañas alinean el título a la izquierda y reservan una zona fija para la `×`; un clic abre archivos y alterna expandir/contraer carpetas. Build Linux y CTest pasan; falta revisión visual real.
- Acciones principales agrupadas en menús estándar de Studio y compartidas con la barra de acceso rápido. Los atajos se editan en `Edición → Configurar atajos`, se validan contra duplicados y persisten con `QSettings`.
- La barra de acceso rápido usa iconos con tooltips y atajos; la lista lateral asigna iconos temáticos a sus 15 secciones. Los iconos usan el tema del sistema con fallback Qt.
- El explorador muestra iconos de `QFileSystemModel` y filtra nombres en el árbol. `Ctrl+Shift+F` busca nombres/rutas indexados o contenido con límites por tamaño; al elegir una coincidencia abre el archivo en la línea. Studio conserva recientes y restaura archivos limpios/grupos divididos con validación de rutas. `Ctrl+Shift+O` ofrece navegación sintáctica de declaraciones comunes.
- La barra de acceso rápido controla la visibilidad de Explorador, Secciones/acciones y Actividad con acciones checkables de los dock widgets. Las secciones operativas presentan estados en el centro; Skills marca habilitadas/recomendadas y Agentes disponibles/en uso en Studio.
- Los logos de `assets/` se empaquetan en recursos Qt: variante mínima como icono de aplicación/ventana y logo completo en la vista Marca.
- Scrollback integrado en `VtTerminalWidget`: conserva hasta 5000 líneas con celdas/estilos ANSI, rueda para navegar, `Shift+PageUp/PageDown` por página y `Ctrl+Home/End` para inicio/final. Muestra badge al revisar historial y vuelve al final cuando se escribe. Conserva rueda como mouse input cuando la app activa modo mouse o alternate screen. `cmake --build build/native --parallel 2` pasó; falta prueba visual.
- Terminal visible añadida a Studio: botón `Terminal +` y `Ctrl+Shift+T` abren el shell del proyecto como pestaña integrada con PTY/ConPTY y libvterm, previa a una consulta bridge de solo lectura que no depende de Zellij. Attach a Zellij sigue externo. Build macOS reportado y build Linux local pasan; falta comprobación visual.
- Corregido error de compilación reportado en macOS: `main_window_agent_sessions.cpp` incluye la definición de `BridgeClient` que necesita para consultar `isRunning()`. Quitada captura lambda no usada en el filtro de secciones. `cmake --build build/native --parallel 2` completó y enlazó `chxchx-studio`.
- Primera pasada de modernización visual de Studio: paleta azul noche/cian, controles con estados hover/focus/selección, separadores y tabs más claros; navegación lateral ahora muestra proyecto y filtra secciones al escribir. Pendiente revisar visualmente y seguir simplificando tareas frecuentes.
- El bridge JSON está descrito por `src/chxchx_tech_lead/workspace/bridge_contracts.py`; las terminales integradas son una capacidad nativa separada. Agentes y terminal nueva de workspace usan PTY/ConPTY; attach a Zellij sigue externo. El siguiente alcance es attach embebido a Zellij.
- Studio ahora valida `schema_version` del bridge antes de procesar un payload y muestra un mensaje legible ante versiones incompatibles, versión ausente/inválida o un nombre de schema desconocido. La versión aceptada por Qt se declara como `BridgeSchemas::Version = 1`; los campos JSON adicionales siguen siendo ignorados por los accesos selectivos existentes.
- Transporte PTY/ConPTY añadido a `native/src/integrations/pty_session.*`: shell o proceso por argv/cwd, entrada, salida asíncrona, resize y cierre. POSIX usa `forkpty` (Linux/macOS); Windows usa ConPTY, mínimo Windows 10.
- `native-pty-smoke` comprueba un shell interactivo: Studio envía una línea al proceso y cambia la geometría; en Linux también valida que `stty size` devuelve el nuevo tamaño. CMake compila y CTest pasa 2/2 en Fedora. El backend ConPTY debe quedar validado por el runner de Windows CI.
- Commit `a5c0753` integra pestañas de agentes con PTY/ConPTY y libvterm. El commit posterior añade `agent preflight` para trust/RAM agregado y pasa el arranque embebido por `agent_pane_command`, incluyendo instrumentación y el prompt contextual de sesión nueva. Python: 195 passed, 1 skipped. El build Linux actual compila y enlaza Studio.
- Cierre del bloque actual: Recursos de Studio consume `bridge resources PATH`, con muestra global del sistema y resumen de procesos gestionados para cada proyecto registrado. El endpoint es read-only y una prueba verifica que conserva el archivo de configuración.
- El editor usa Scintilla/Lexilla fijados a commits upstream; tiene UTF-8, guardado atómico, lexer por extensión y búsqueda en archivo. `workspace terminal` abre una pestaña integrada; attach a Zellij y `agent attach` mantienen launcher externo, con preview/confirmación y preflight de RAM para agentes.
- Validación actual: `python -m compileall -q src tests scripts`; suite Python (`195 passed, 1 skipped`); `git diff --check`; `cmake --build /tmp/chxchx-studio-post-pull --parallel 2`; CTest (`2/2`); inicio Qt offscreen durante 3 s (timeout 124 esperado). Qt6 Core5Compat está instalado; CMake descargó Scintilla/Lexilla/libvterm y compiló el proyecto tras recuperar conectividad.
- Requisito local: el build estándar usa Qt6 Core5Compat de desarrollo; ya está instalado en este Fedora. CI instala el módulo desde sus runners.
- Pendiente inmediato: QA visual/interactivo en un proyecto real, smoke de restauración de sesión, prueba UI del diff Git y validación manual de Zellij/VT. Después: evaluar LSP, medir rendimiento, empaquetar y verificar cada plataforma con evidencia real.
- Studio añade vistas funcionales de Configuración, Guía y Marca. Setup ofrece preparación completa, init completo/mínimo, instalación de herramientas e integración MCP; todas las mutaciones pasan por dry-run y confirmación. Doctor es solo lectura. `Ctrl+P` filtra vistas, archivos y acciones disponibles; `Ctrl+F` busca en el archivo abierto.
- CI ahora compila Qt en Ubuntu, Windows, macOS y Fedora. Build local Qt 6.11.2, compileall y suite: 191 passed, 1 skipped. `setup --dry-run` y `doctor` se ejecutaron correctamente sin cambios.
- La rama activa es `dev`.
- Primer corte de ChxChx Studio creado en `native/`: aplicación C++20/Qt 6 Widgets con árbol de proyecto, apertura de archivos, pestañas, guardar atómico y vistas con consultas asíncronas para Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs, Recursos, Handoff, Memoria, Chats y Errores.
- Añadido `chxchx-tech bridge status PATH`, contrato JSON `chxchx.project-status` versión 1 con metadatos de proyecto, trust/workspace, agentes, procesos, RAM/swap/CPU y política RAM Governor. El snapshot evita persistir cambios de estado de procesos; Studio lo consume y presenta vistas resumidas por área.
- Lecturas de Inicio, Proyecto, Agentes, Procesos y Recursos ya usan el bridge JSON en vez de parsear tablas Rich del CLI. `bridge status .` devuelve JSON válido en ejecución local.
- Agentes y Procesos tienen selector y acciones nativas: iniciar agente, iniciar todos, nuevo chat con contexto, iniciar/detener proceso. Cada acción primero corre en `--dry-run` y pide confirmación; el inicio real de agentes vuelve a pasar por el RAM Governor y solo ofrece `--force` tras mostrar el aviso actualizado y pedir confirmación.
- El bridge ahora incluye procesos configurados detenidos para que se puedan seleccionar e iniciar. La navegación que cambia mientras hay una consulta en curso queda en cola y se actualiza al terminar.
- Workspace ahora ofrece trust, iniciar/reanudar, suspender y detener procesos con preview/confirmación. Proyectos muestra los registros disponibles y puede cambiar el workspace con `projects switch --no-attach`, actualizar raíz del árbol de archivos y cwd de Studio, y repetir la confirmación del RAM Governor con el estado vigente.
- Proyectos también permite dar trust al proyecto elegido antes de cambiar; deshabilita Switch para rutas no confiables y para el proyecto actual.
- `bridge status` incluye proyectos registrados, estado, trust y existencia de cada ruta. El payload JSON v1 se amplió de forma aditiva y reporta configuración inválida sin ocultar la lista multiproyecto.
- Skills y Tech Packs ya usan ese bridge: Studio muestra las 21 skills, selección por proyecto, recomendaciones para el stack actual y los 13 packs con composición/motivos de detección. Permite habilitar/deshabilitar una skill, aplicar un pack o los detectados y sincronizar el contexto; todas las mutaciones hacen preview `--dry-run` y piden confirmación antes del comando real.
- Una prueba nueva cubre catálogo, recomendaciones, composición de packs y reflejo de selección activa en el payload. Validación: `python -m compileall -q src tests`, suite (`189 passed, 1 skipped`), `git diff --check HEAD` y dry-runs de `skill enable`, `skill sync` y `pack apply-detected` pasan sin cambios reales.
- Qt 6.11.2 quedó instalado en el host Fedora. `cmake -S native -B /tmp/chxchx-studio-build -G Ninja` configura y `cmake --build /tmp/chxchx-studio-build --parallel 2` compila y enlaza. El primer build reveló el uso inválido de `QFileSystemModel::isFile`; se cambió a `!isDir` y el build final pasó.
- Handoff y Memoria ya tienen vistas nativas propias. Handoff carga el documento, precarga los campos editables cuando encuentra el formato administrado y guarda resumen/pendiente/validación mediante `workspace handoff` tras preview y confirmación. Memoria filtra notas por título/contenido y muestra su contenido como solo lectura.
- Se agregaron lecturas acotadas `bridge handoff` y `bridge memory`; las rutas resueltas deben permanecer dentro del proyecto. Se validaron ambos contratos y `workspace handoff --dry-run`. Suite de ese bloque: `190 passed, 1 skipped`.
- Chats y Errores ya tienen vistas nativas de solo lectura. Chats filtra Codex/Claude por proyecto, título o contenido y obtiene la transcripción seleccionada bajo demanda. Errores lista hasta 200 entradas recientes filtradas por la ruta exacta del proyecto.
- Se agregaron `bridge chats`, `bridge conversation` y `bridge errors`. Pruebas cubren mensajes e historial acotados al proyecto y errores de otro proyecto excluidos. CLI smoke: contratos de conversaciones y errores válidos. Suite completa actual: `191 passed, 1 skipped`; `python -m compileall -q src tests` y `git diff --check HEAD` pasan. `QT_QPA_PLATFORM=offscreen timeout 3s /tmp/chxchx-studio-build/chxchx-studio .` mantuvo la app abierta durante el smoke de inicio; timeout 124 esperado.
- CLI smoke en `--dry-run` de trust, start, suspend, resume, stop y switch terminó con código 0, sin ejecutar operaciones reales. `python -m compileall -q src tests`, `git diff --check HEAD` y la suite (`188 passed, 1 skipped`) pasan. El contrato se comprobó con `project_status_payload`: esquema `chxchx.project-status` v1, cuatro proyectos registrados y estado de trust.
- Smoke seguro: CLI `agent start codex --dry-run`, `agent start --all --dry-run` y `agent start codex --new-chat --dry-run` terminaron con código 0; no se iniciaron agentes. No se probó el dry-run de procesos porque este proyecto no configura procesos.
- `README.md` documenta requisitos y comandos de build. Se fijó Qt mínimo 6.4 para ampliar compatibilidad con toolchains de Ubuntu/Fedora.
- Las consultas CLI usadas por el shell se ejecutaron en este proyecto y terminaron con código 0. `git diff --check HEAD` pasa.
- Contrato comprobado por CLI más parseo JSON: `chxchx-tech bridge status .` entrega `schema=chxchx.project-status`, versión 1 y los cinco grupos esperados (`project`, `workspace`, `agents`, `processes`, `resources`). `python -m compileall -q src` pasa; no se ejecutó la suite.
- Build nativo Linux validado con Qt 6.11.2 y Scintilla/Lexilla; Windows/macOS y otras distribuciones dependen de CI.
- La decisión de producto cambió: la interfaz/editor será una aplicación nativa C++20 + Qt 6 Widgets + Scintilla/Lexilla, multiplataforma macOS/Windows/Fedora/Ubuntu y con paridad funcional completa con la TUI. `.ai/EDITOR-PARITY.md` contiene la matriz de funciones y plan de entregas.
- El estado objetivo es un núcleo C++ común a la app, CLI y TUI. Se iniciará con un bridge JSON temporal a servicios Python para conservar paridad; CLI/TUI delegarán por dominios al core nativo al validar cada migración.
- Qt 6.11.2 y CMake/GCC/Ninja están disponibles en este entorno. Para Core5Compat se usaron archivos de desarrollo extraídos bajo `/tmp` por falta de privilegios de instalación.
- CLI y TUI consumen tokens compartidos de color para estados y acentos. Se agregó `editor setup [PATH]` para generar `.ai/sublime/<proyecto>.sublime-project` con exclusiones comunes, `--dry-run`, escritura idempotente y backup al reemplazar una configuración administrada. `--no-open` deja solo el archivo.
- Validación tras esta integración: `python -m compileall src tests`, suite completa (`188 passed, 1 skipped`) y `git diff --check HEAD` limpios.
- Verificación del RAM Governor inicial: `python -m compileall src tests`, suite completa (`184 passed, 1 skipped`) y `git diff --check HEAD` limpios. Todos los módulos de producción quedan bajo 300 líneas.
- RAM Governor inicial: `workspace.resources` ahora incluye `max_agents` (default 2); proyectos nuevos usan aviso RAM 70%, crítico 85% y aviso swap 40%. La TUI calcula cuántos agentes nuevos se abrirían, muestra un diálogo al superar umbrales o presupuesto y deja cancelar o continuar. `agent start`, `run` y `projects switch` hacen el preflight en CLI y requieren `--force` para confirmar. No mata procesos automáticamente.
- El panel Recursos muestra los umbrales y la cuota. El comando `chxchx-tech resources` también imprime esta política. Configs existentes mantienen sus umbrales explícitos; en este repo siguen en 75%/90% y reciben `max_agents = 2` por default al cargar.
- Pruebas nuevas validan configuración, avisos de RAM/cuota y cancelar o confirmar desde el diálogo TUI.
- Se añadió `ui-ux-design`, una guía avanzada de diseño de producto UI/UX; se recomienda en proyectos React, Next.js y React Native y ya forma parte de esos Tech Packs. El catálogo se amplió después a 21 skills.
- La TUI tiene una pestaña principal `Skills` con catálogo buscable y Tech Packs. Muestra total de skills, selección activa, packs detectados, razones de recomendación y composición de cada pack.
- Desde la TUI se puede habilitar/deshabilitar skills, aplicar un pack o todos los packs detectados y sincronizar `.ai/SKILLS.md`/`AGENTS.md`; cada escritura preflighta con `dry-run` y las operaciones existentes hacen backup.
- La vista mantiene dos subpestañas, Catálogo y Tech Packs, para que las acciones sigan accesibles en una terminal pequeña; se puede abrir desde `Ctrl+P` buscando “skills”.
- Validación tras integrar la TUI: `python -m compileall src tests` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (180 passed, 1 skipped); `git diff --check HEAD` limpio.
- Inicio de v0.6: existe `SkillRegistry` con metadatos TOML e instrucciones Markdown, y el CLI expone `skill list/search/info/recommend`.
- Catálogo en ese checkpoint: 17 skills generales y para Python, TypeScript, React/Next, UI/UX, NestJS, .NET, React Native, PostgreSQL, Prisma, Redis y Docker; `recommend` usa `detect_project` para comparar stacks y lenguajes.
- `docs/` sigue presente en esta copia local y está excluida del índice de Git; `.gitignore` evita volver a agregarla.
- Basic Memory sigue apuntando a otro proyecto en esta sesión; no se guardó información allí.
- Validación actual del Skill Registry y Tech Packs: `python -m compileall src tests` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (179 passed, 1 skipped); pruebas CLI cubren catálogo, estado, recomendaciones, detección y aplicación aditiva con dry-run.
- El catálogo creció a 17 skills para stacks detectados y guías generales; el detector reconoce React, Prisma, Redis y Docker además de sus marcadores anteriores.
- `skill enable/disable` guarda selección local por proyecto, `skill sync` genera `.ai/SKILLS.md` con bloque administrado y referencia desde `AGENTS.md`; las mutaciones ofrecen `--dry-run` y backups.
- Tech Packs TOML componen skills; `pack list/detect/info/apply` explica coincidencias y agrega skills de forma aditiva. `skill recommend` expone razones basadas en stack/lenguaje o guía general.
- Se endurecieron tres pruebas asíncronas de la TUI para esperar hasta 5 segundos la finalización de workers en CI; antes usaban ventanas fijas de 200–400 ms.
- Verificación local del ajuste: `python -m compileall src` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- El test de recuperación de procesos parchea el terminador del árbol en Windows y el terminador POSIX en otros sistemas, evitando invocar `taskkill` durante el test.
- Ese test usa ahora un handle simulado y no inicia procesos reales; verifica la recuperación y el despacho al terminador de plataforma sin bloquear el CI de Windows.
- Inicio ofrece acceso directo a Terminal (`T`) y Agentes (`A`).
- La pestaña `Agentes` apila a todo el ancho el encabezado y el panel de uso; debajo acomoda los panes de agentes según `workspace.layout.orientation`.
- En sesiones ya existentes sin pestaña `Agentes`, su layout y panes se crean mientras el cliente Zellij está adjunto; el foco se aplica después de la conexión.
- El panel de uso recibe `--root` sin comillas duplicadas.
- Codex y Claude iniciaron en una prueba real; Claude mostró su confirmación de confianza propia para la carpeta.

## CI opcional para proyectos de usuario

- ChxChx genera de forma opt-in GitHub Actions para Python, Node.js y CMake/C++/Qt; el setup ofrece `--dry-run`, conserva workflows no administrados y verifica el hash de los administrados antes de actualizarlos o retirarlos. Los reemplazos/retiros dejan backup local fuera del repositorio.
- `ci status/run/logs` usa GitHub CLI oficial sin almacenar credenciales; TUI y Studio presentan estado y confirmación de escrituras, dispatch y retiro. `bridge ci` es lectura JSON v1.
- El template CI tiene `push`, `pull_request`, `workflow_dispatch`, acciones fijadas por SHA y `contents: read`. El workflow debe subirse al repositorio para activarse.
- CD optativo de proyectos de usuario está implementado para Docker Compose en VPS mediante GitHub Actions y SSH. CLI/TUI/Studio comparten setup, estado, dispatch manual confirmado, logs y retirada; `bridge deploy` es lectura JSON v1. Modo manual es el valor inicial; `--auto-deploy-on BRANCH` opta por CD en push.
- El workflow usa Environment `production`, variables/secretos externos, host key SSH verificada estrictamente, clon VPS con remote coincidente y working tree versionado limpio, `docker compose up -d --build`, revisión de servicios/health y rollback al commit previo si falla. Se deja `.env` y volúmenes en el VPS; el rollback no revierte datos/migraciones.
- Alcance inicial: un manifiesto Compose en la raíz y servicios de larga duración. Requiere preparar en el VPS el clon, Docker Compose y permisos del usuario dedicado; para repos privados, el clon necesita su propia llave de lectura. No se crean ni guardan credenciales.
- La publicación preview del propio ChxChx queda aplazada a petición del usuario. El workflow `.github/workflows/ci.yml` puede cambiar para validar y entregar los instaladores; `publish-studio-preview` permanece limitado a `dev` y no se publicó desde esta rama.
- Se agregó `psutil>=7,<8` como dependencia para que la telemetría de memoria/procesos funcione en Windows/macOS además de Linux. El CI detectó y corrigió un include de `QJsonDocument` que Qt 6.8 exigía explícitamente.
- Validación local: `python -m compileall src tests scripts`, pytest (`227 passed, 1 skipped`), build Studio C++/Qt, CTest (`6/6`), CLI `ci --help`, `bridge ci .` y `git diff --check`.
- Publicado en `dev`: commits `32843ba`, `ad1ed5e`, `9b572ab`, `73a220e`, `064f88f`, `d6f266d`, `5db1736` y `cb86b3d`. Las 9 combinaciones de pruebas Python/OS pasan. CI detectó que el smoke PTY usaba `powershell.exe` no disponible por nombre en el runner y que la prueba visual de VT no generaba suficiente historial para el tamaño de página Windows; ambos supuestos se corrigieron (`COMSPEC` y 120 líneas). El run `38684041970` confirmó que Linux, Fedora, las pruebas Python y el smoke VT pasan; el smoke PTY Windows no recibió `READY` y cmd reportó que no encontraba la ruta. El cambio local usa un proceso hijo del mismo ejecutable para probar ConPTY sin depender del shell.
- Las cargas nativas usan `compression-level: 0` porque publican paquetes Qt grandes. GitHub Actions terminó builds de Ubuntu/Fedora y antes comprobó compilación en Windows/macOS; falta completar el run del último commit y la publicación del preview de Studio.
- El run `38685336827` encontró un error de compilación solo en Windows (`startError` sin declarar); se restauró la variable. `38685991849` compiló Windows, pero CTest mostró que la salida del proceso hijo escapaba al log. El launcher declara `STARTF_USESTDHANDLES`; `38686726655` confirmó que la salida y la entrada llegan por ConPTY, aunque el smoke no observaba el fin del proceso porque el lector esperaba EOF con la pseudoconsola aún abierta. El watcher ahora cierra la pseudoconsola al acabar el proceso, drena la salida y después emite `processFinished`; falta validar el ciclo completo en CI.
- Build Qt y CTest Linux pasan (`6/6`); pytest pasa (`227 passed, 1 skipped`). Los jobs completados de Ubuntu, Fedora y Python Windows/Linux están verdes. El diagnóstico de CTest usa PowerShell con `continue-on-error` y reportó los detalles de los dos smoke Windows.
- Contexto anterior de CI del producto: los detalles de runs enumerados arriba corresponden al checkpoint previo a aplazar la publicación de ChxChx; su estado remoto más reciente no está consultable desde esta sesión.
- Validación local de esta integración: compileall de `src/tests/scripts`, pytest (`247 passed, 1 skipped`), build Studio C++/Qt, CTest (`6/6`), CLI `deploy --help`, JSON `bridge deploy .`, `bash -n` sobre el runner y el script remoto generados, y `git diff --check` pasan. No se ejecutó un despliegue real porque requeriría un VPS configurado.
- El CI remoto se disparará desde una rama feature autorizada, nunca desde `dev`: el push a `dev` puede publicar el preview del producto. La publicación del propio ChxChx queda aplazada; en esta rama el workflow solo valida y empaqueta assets, y el job de publicación no se activa.
- Push `5c17e46` a `feature/vps-compose-deploy` disparó el run `38692862282` (#86). La matriz Python (Ubuntu/macOS/Windows, 3.11–3.13), wheel y builds/CTest nativos de Fedora, Ubuntu y Windows terminaron correctamente. macOS ARM compiló y pasó CTest, pero su job falló al publicar el artifact; macOS Intel seguía en la carga del artifact al último chequeo. El workflow de publicación está limitado a `dev` y no se activó.

- CI run `38728824695` (#87), commit `9565feb`: 15 jobs aplicables finalizaron correctamente. CPack ya se valida y solo se carga el ZIP/DMG/TGZ final, no su staging temporal; macOS ARM e Intel validaron el contenido y subieron el artifact. Windows validó el script PowerShell y su paquete; Ubuntu/Fedora también validaron sus TGZ. `publish-studio-preview` quedó omitido porque la rama no es `dev`.

## Estado del runtime observado

- La sesión guardada `chichan-tech-lead` apareció `EXITED` y su metadata vieja mostraba panes de agentes sin filas de contenido. El inicio del workspace recrea sesiones `EXITED`; para una sesión viva sin pestaña `Agentes`, el flujo nuevo la agrega al adjuntarse.

## Studio: conservar pestañas de trabajo al cambiar de sección

- Las vistas de sección comparten una pestaña fija con documentos, terminales y agentes. Navegar entre secciones solo cambia el contenido de esa pestaña y conserva abiertas las demás.
- La pestaña fija refleja el nombre/icono de la sección activa, no muestra botón de cierre y está protegida en `closeEditorTab`.
- Build Linux y CTest (2/2) pasan; queda QA visual/interactivo en escritorio.

## Studio: contraste de texto del editor

- El texto base del editor bajó de blanco azulado a un gris azulado suave (`#B9C6D8`); los tokens de sintaxis azul claro también se atenuaron (`#98B5CC`). Se mantienen intactos el fondo, la selección y el color de números de línea.
- Build Linux completado; pendiente revisar visualmente el contraste en escritorio.

## Studio: margen y ajuste de línea

- El margen de números usa un fondo azul marino `#122941` y primer plano `#7896B5`; se reaplican tras la inicialización del lexer para que no vuelva al gris predeterminado.
- El ajuste de línea está activo de forma predeterminada, oculta la barra horizontal y se alterna desde `Ver → Ajuste de línea` o `Alt+Z`. La preferencia queda guardada en `QSettings` y aplica a documentos abiertos y nuevos.
- Build Linux completado; falta revisión visual e interacción con un archivo largo.

## Studio: pestañas compactas

- La pestaña de secciones queda sin texto y conserva solo el icono de Inicio; su tooltip indica que abre las secciones del proyecto.
- Los tabs de archivos, terminales y agentes reducen ancho máximo/mínimo, altura, padding, margen y tamaño de cierre manteniendo el título legible.
- Build Linux y `git diff --check` pasan; falta validar el tamaño visual en escritorio.

## Studio: nombres de pestañas

- La pestaña Inicio usa mínimo de 44 px y el icono de casa a 14 px. Las pestañas de código muestran el basename con extensión (por ejemplo, `main_window.cpp`); el tooltip mantiene la ruta completa.
- Las terminales integradas se nombran `Terminal #1`, `Terminal #2`, etc. durante la sesión de Studio.
- Build Linux y `git diff --check` pasan; falta QA visual del usuario.

## Studio: grupos de pestañas divididos

- El centro usa un `QSplitter` con dos grupos de pestañas. Clic derecho sobre una pestaña permite dividirla a la derecha o abajo y moverla entre grupos; sirve para archivos, terminales y sesiones de agente.
- Cerrar la división devuelve sus pestañas al grupo principal sin cerrar procesos o sesiones. Inicio permanece fijo en el grupo principal; las acciones de archivo y ajuste de línea operan sobre el grupo activo.
- La Guía rápida describe el menú contextual. Build Linux y `git diff --check` pasan; pendiente probar las interacciones de split en escritorio.

## Studio: acciones de edición y atajos

- `Edición` ofrece Deshacer, Rehacer, Cortar, Copiar, Pegar y Seleccionar todo con atajos estándar de Qt; quedan editables desde Configurar atajos.
- Las acciones se despachan al editor Scintilla, campos QLineEdit, paneles QPlainTextEdit/QTextEdit o terminal con el foco. En terminal se conserva Ctrl+C como interrupción; Pegar envía el portapapeles como texto.
- La Guía rápida explica estas acciones. Build Linux, inicio offscreen y `git diff --check` pasan.

## Studio: recorrido guiado para preparar un proyecto

- La sección Guía presenta cuatro pasos en orden: preparar el proyecto con Init, elegir y sincronizar skills, autorizar/iniciar el workspace y abrir una nueva sesión de agente con contexto.
- Cada tarjeta tiene un botón que navega directamente a Configuración, Skills, Proyecto o Agentes. La guía también resume el uso diario del explorador, Terminal +, divisiones de pestañas y configuración de atajos.
- Validación: build Linux en `/tmp/chxchx-studio-post-pull`, inicio offscreen por 3 segundos y `git diff --check` pasan. Falta revisión visual e interacción en escritorio.

## Revisión para empezar a usar Studio

- Roadmap y matriz de paridad actualizados: el plan anterior aún describía Sublime como editor objetivo y marcaba la búsqueda de rutas y los splits como pendientes. Ahora distingue funcionalidades listas, bloqueos para comenzar a usarlo y mejoras de v1.0.
- El cierre mínimo es QA interactivo en un proyecto confiable: Init/trust, skills, edición/guardado, terminal, agente/contexto, RAM y cierre limpio. Los builds/smokes por sistema requieren evidencia real; que exista un job CI no demuestra que pasó.
- Cambios de Studio y limpieza documental quedaron comprometidos en dos commits de `dev`. `docs/` permanece con 18 archivos en este checkout y está ignorada; otros clones no la recibirán. No se ejecutaron pruebas en la revisión documental; el build/CTest de Studio constan en el handoff anterior.

## Studio: acciones rápidas centrales en Inicio

- Inicio incorpora botones centrales para trust, iniciar/reanudar, suspender y detener workspace; terminal; Init completo, sincronización de skills y Doctor; selector de agente con abrir/nueva sesión/abrir todos.
- Los comandos reutilizan los previews y confirmaciones existentes; agentes conservan preflight de trust/RAM. El panel lateral sigue disponible, pero no es requisito para estas acciones frecuentes.
- El usuario revisó el panel en Studio y confirmó que la presentación le gusta. Compilación Linux y CTest 2/2 pasan; `python -m compileall src` pasa y pytest reporta 195 passed, 1 skipped. No se ejecutaron mutaciones de Init/workspace durante QA.

## Terminal Studio: smoke de render VT y scrollback

- `native-vt-terminal-smoke` usa Qt offscreen y libvterm para comprobar salida ANSI roja, desplazamiento al historial, avance por página y retorno al final en una terminal con más de 24 líneas.
- La búsqueda global (`Ctrl+Shift+F`) ofrece modo de contenido asíncrono sobre el índice de rutas, omite binarios, abre coincidencias en su línea y limita el escaneo a 1 MiB por archivo, 64 MiB por consulta y 100 resultados.
- Studio recuerda hasta 20 archivos recientes por proyecto; al cerrar guarda archivos limpios de ambos grupos y al iniciar restaura solo los que aún existen dentro de la raíz canónica. Terminales y agentes no se reabren.
- `Herramientas → Diff Git del archivo actual` ejecuta `git diff` con lista de argumentos desde el proyecto y presenta salida de solo lectura limitada a 2 MiB. Compara el archivo rastreado con `HEAD`; no incluye untracked ni buffers sin guardar.
- `Navegar → Símbolos del archivo actual` (`Ctrl+Shift+O`) lista declaraciones comunes de Python, C/C++, JavaScript/TypeScript, Java y C#, y salta a la línea. Es sintáctico por líneas; no resuelve referencias ni reemplaza LSP.
- Validación actual: build Qt Linux, CTest 6/6, arranque offscreen por 3 segundos, `python -m compileall src`, pytest 195 passed/1 skipped y `git diff --check`.
- El arranque dentro del sandbox aborta al usar XCB/Wayland, mientras que `minimal`/`offscreen` funcionan. Con acceso elevado a la sesión de escritorio, Studio abre y permanece activo para revisión manual. El entorno restringe ptrace, así que no hay backtrace del intento dentro del sandbox.
- Muestra preliminar en el checkout actual: ventana gráfica en reposo tras 3:46, RSS 119400 KiB (~116.6 MiB), CPU promedio 0.4%; un arranque offscreen de 10 s alcanzó 63660 KiB (~62.2 MiB) de RSS máximo. Falta medir startup y latencia de navegación con metodología repetible y definir límites.
- Baseline de `ProjectFileIndex` con cinco ejecuciones por escenario: el checkout (341 archivos) indexó en 4 ms; búsqueda de ruta sin coincidencia p50 0.392 ms/p95 0.404 ms y búsqueda de contenido “TODO” 15–16 ms, 34 coincidencias y truncamiento señalado por archivos sobre 1 MiB. Una fixture sintética de 10.000 archivos (100 carpetas × 100 archivos) indexó en 62–65 ms; una búsqueda de ruta sin coincidencia tuvo p50 mediano 9.00 ms/p95 mediano 9.55 ms, y escaneo de contenido sin coincidencias 99–100 ms sin truncamiento. El harness alcanzó 14.3–14.8 MiB RSS. Es una medición del índice, no de primera imagen ni memoria de Studio; no son límites de aceptación.
- Build Linux correcto y CTest 3/3 pasa (editor, PTY y VT). `git diff --check` limpio.
- CTest adicional comprueba resize con historial visible; un proceso PTY escribe 80 líneas, recibe entrada de teclado y termina correctamente. Sigue pendiente QA visual del reflow del historial en una sesión real.

## Studio: attach integrado a Zellij

- La paleta abre `chxchx-tech workspace attach PATH` dentro de una pestaña PTY/libvterm, después del preview y confirmación existentes. Se conserva otra acción de paleta para lanzar el cliente Zellij en una terminal externa.
- `WorkspaceTerminalWidget` acepta comandos arbitrarios manteniendo shell por defecto; `BridgeClient` comparte la resolución de la CLI (`CHXCHX_TECH_CLI`, `.venv` local o `PATH`).
- Build Linux y CTest 4/4 pasan, incluyendo un smoke offscreen que verifica que una pestaña PTY ejecuta y termina un comando personalizado. Falta validar attach real/foco/cierre con Zellij en escritorio.

## Studio: empaquetado CPack

- CMake configura ZIP para Windows, DMG para macOS y TGZ para Linux. El paquete incluye los avisos originales de Scintilla, Lexilla y libvterm junto con `THIRD_PARTY.md`.
- Estado histórico previo: CI solo publicaba el TGZ Fedora y no dependía de las matrices Windows/macOS/Ubuntu. El flujo actual está descrito al inicio del archivo y en ADR-0005.
- Validación local: CMake configure/build, CTest 6/6, CPack generó el TGZ Linux y el archivo contiene binario y licencias; `python -m compileall -q src tests scripts`, pytest (195 passed, 1 skipped) y `git diff --check` pasan.
- Pendiente: ejecución real del workflow en Ubuntu/Windows/macOS, prueba de instalación/arranque de los artefactos, QA manual en el proyecto real y mediciones repetibles de startup/navegación.

## Instalación Fedora anterior (histórica; sustituida por el instalador multiplataforma)

- El instalador Fedora dedicado ahora delega en `scripts/install-studio.sh`. El tag antiguo `studio-fedora-dev` no forma parte del flujo de release actual; usa las instrucciones multiplataforma de README.

## Memoria explícita al iniciar chat nuevo

- `agent_pane` revisa transcripts Codex/Claude y exportaciones sanitizadas de OpenCode al terminar sesiones, y guarda solicitudes explícitas en `.ai/memory/PROJECT_MEMORY.md`. OpenCode se consulta con `opencode session list` y `opencode export --sanitize`, filtrando por la ruta exacta del proyecto. Antes de iniciar `--new-chat`, revisa la memoria local del proyecto, evita duplicados y adjunta las entradas recuperadas al prompt.
- Excluye preguntas que piden recuperar una frase anterior, mensajes de asistente y mensajes que coinciden con indicadores de secretos; no corre al iniciar agentes normalmente.
- Validación: `python -m compileall -q src tests`; suite Python `201 passed, 1 skipped`; `git diff --check` limpio.
- Pendiente: probar en Studio un mensaje exacto de memoria, cerrar el proyecto, iniciar chat nuevo y pedir la frase. La recuperación depende de que el transcript local del proveedor exista y contenga el mensaje original.
- Ajuste por prueba: el agente consultó `chxchx-studio-qa` en Basic Memory. `new_chat_prompt()` ahora lee `memory_project` del `.ai/chxchx-tech.toml` de la raíz activa y ordena pasar ese nombre exacto como argumento `project`; prohíbe IDs de otros proyectos y búsquedas globales. Si no hay ámbito configurado, indica no usar Basic Memory.
- Una pregunta de recuperación como “¿cuál fue la frase?” nunca se guarda como memoria; la captura solo acepta mensajes del usuario con una solicitud de recuerdo y excluye preguntas que intentan recuperar información anterior.

## Memoria resiliente de agentes

- Las reglas administradas del proyecto piden guardar solicitudes explícitas de “recuerda esto” en `.ai/memory/PROJECT_MEMORY.md`, conservar frases/nombres literalmente, no guardar secretos o datos sensibles y consultar las entradas pertinentes al iniciar una tarea.
- El prompt de “Nueva sesión” de Codex, Claude y OpenCode carga esa nota e indica guardar la petición en el momento. Configuración conecta Basic Memory/Serena a OpenCode mediante la CLI oficial y alcance de proyecto; `.ai/memory/` permanece como memoria local resiliente.
- El cierre de Studio sigue sin reabrir el proceso del agente ni restaurar su transcripción. La persistencia está en el archivo del proyecto; solicitudes de memoria anteriores que nunca se guardaron no se recuperan automáticamente.
- Validar manualmente en una conversación nueva: pedir una frase no sensible, confirmar que el agente informa la ruta, cerrar/reabrir Studio y pedir que la repita. No ejecutar esta prueba sobre información privada o secretos.
