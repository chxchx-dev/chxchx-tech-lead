# Current State

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## README y ramas

- `main` se integró en `dev` (`cd86d46`) y los 10 commits de `feature/reliable-chat-memory` también quedaron integrados (`9ceb081`): memoria por chat, diario/compactación de contexto, chat visual y mejoras de Studio.
- README conserva sus instrucciones y ahora abre con el arte ASCII solicitado, badges y enlaces rápidos a Studio, TUI y CLI.
- El árbol está en `dev`, 15 commits por delante de `origin/dev`; no se ha hecho push. La rama local de feature se eliminó tras comprobar el merge; `origin/feature/reliable-chat-memory` se conserva como respaldo hasta publicar `dev`.

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
- Distribución: CPack genera ZIP/DMG/TGZ y un `Setup.exe` NSIS. Existe el release rodante `studio-dev-preview`. El run #87 de CI validó los paquetes en todos los runners. Nunca se ha probado la instalación en máquinas limpias.
- Memoria: `.ai/memory/PROJECT_MEMORY.md` guarda las peticiones explícitas, y Basic Memory se usa limitado al `memory_project` del proyecto. Al cerrar cada sesión de Codex/Claude se escribe un resumen en `.ai/history/SESSIONS.md`, que se inyecta en el siguiente chat nuevo. `CURRENT_STATE`/`HANDOFF` se compactan solos hacia `.ai/history/`.
- Lo que bloquea el uso diario: QA interactivo de Studio en Fedora con un proyecto real. El orden de trabajo está en `.ai/ROADMAP.md` (plan de noviembre 2026).
- El equipo actual (Fedora, noviembre 2026) no tiene cmake, g++ ni qt6-qtbase-devel, así que Studio no se compila aquí. El estado anterior completo está en `.ai/history/CURRENT_STATE.md`.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.
