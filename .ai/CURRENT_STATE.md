# Current State

## Studio: primeros pasos y scroll (2026-11-15)

- Versión preparada: `0.6.1` en Cargo y en el paquete Python auxiliar; el changelog describe el ajuste de Inicio.
- Inicio ahora muestra tres pasos concretos (preparación, confianza y empezar a trabajar), con lenguaje más directo. Al terminar de calcular una vista previa, Studio desplaza la página hasta la revisión para que el botón no parezca no responder.
- `cargo check --locked --features studio` y `cargo fmt --all` pasan. La lentitud general del scroll aún requiere medición en la ventana; el cambio reduce la altura de Inicio y arregla la visibilidad de las revisiones, pero no confirma una causa global de render.

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Ajustes de Git: scroll y Sugerir (2026-11-13)

- El editor de mensaje de commit dentro del área con scroll mostraba texto corrupto al
  desplazar (el renderer por software no recorta un `text_editor` dentro de un
  `scrollable`). Ahora la caja está fuera del scroll (`studio_git_changes.rs`). Regla: no
  poner `text_editor` dentro de un `scrollable`; los `text_input` de las tarjetas del plan
  sí se recortan bien (comprobado desplazando la lista en pantalla).
- Sugerir ya no propone un commit único cuando hay varios temas: abre el plan con los
  archivos relacionados repartidos y solo rellena la caja cuando hay un único tema.

## Sugerencias y plan de commits en Git (2026-11-13, Windows)

- El Studio Qt pedía el mensaje y el plan a un agente. En Rust se hizo un motor local y
  determinista (`features/git/suggest*.rs`, `plan.rs`): clasifica archivos, agrupa por
  módulo/sub-módulo/familia, decide el tipo Conventional Commits y redacta el mensaje.
  UI: botón Sugerir, tarjeta «Organizar en commits» y tarjetas editables
  (`studio_git_suggest.rs`, `studio_git_plan_view.rs`, `studio_git_compose.rs`).
- Ejecutar un grupo usa `Repository::commit_paths` (`git add` + `git commit -- rutas`), así
  que no arrastra lo que ya estaba preparado; se verificó en un repositorio real.
- Visual: guía plegable, chips ↑/↓ en la rama, divisores entre secciones, columna fija del
  grafo del historial y avisos de tres tonos (ok, aviso, error).
- Verificado: 58 pruebas (49 unitarias, 6 de catálogo, 3 de producto), Clippy, formato y
  capturas con este repositorio (100+ cambios: 12 commits propuestos).
- Límites: es heurístico (el tipo puede no ser el correcto: se edita con un clic) y no
  entiende la intención del código como un agente. No se probó el botón «Guardar todos»
  contra un historial real del usuario (sí `commit_paths` en repositorios temporales).

## Git en la barra lateral del Studio Rust (2026-11-13, Windows)

- Port del panel Git del Studio Qt: dominio en `features/git/` (tipos, parseo de
  `status --porcelain=v2`, grafo de carriles, guía, errores amigables, operaciones) y
  adaptador `integrations/git.rs` (sin shell, `LC_ALL=C`, sin ventana, límite de salida y
  timeout, un escritor a la vez). UI: `studio_git*.rs` (estado, comandos, marco, cambios,
  filas, historial/ramas, avisos). Diffs en pestañas de solo lectura (`Tab.readonly`, no se
  guardan en la sesión). Explorador y barra de estado muestran el estado Git.
- Seguridad: proyecto sin trust => hooks del repositorio desactivados
  (`core.hooksPath` vacío); `core.fsmonitor` siempre desactivado; descartar guarda copia en
  `.git/chxchx-backups/`; enviar, descartar, eliminar rama/stash y cancelar fusión piden
  confirmación en línea.
- Corregido de paso: 10 sitios del Studio hacían `thread::spawn(..).join()` dentro de código
  asíncrono, bloqueando hilos del ejecutor (`jobs::offload` lo hace sin bloquear).
  También se corrigió el grafo cuando dos líneas apuntan al mismo commit.
- Verificado: 40 pruebas unitarias (13 de Git sobre repositorios reales, incluida la de
  hooks), Clippy, formato y capturas con este repositorio (90+ cambios): cabecera, guía,
  lista, Historial con commit abierto y diff coloreado. No verificado: clics reales sobre
  los botones Git (publicar, enviar, fusionar contra un remoto real), conflictos de merge de
  extremo a extremo, repositorio anidado (prefijo) en la GUI, macOS/Linux.
- No portado: asistente con agentes (sugerir mensaje, «Pedir consejo»), diff en columnas,
  confirmación recordada de push, rebase/cherry-pick/blame. Ver HANDOFF.

## Espacio de trabajo en la barra lateral (2026-11-13, Windows)

- Se eliminó el panel derecho «Secciones» para dar todo el ancho al editor. Las secciones
  viven en la tercera vista de la barra lateral izquierda (`studio_workspace.rs`,
  `shell::Side::Workspace`), agrupadas y con una tarjeta de estado del proyecto. La vista
  activa se guarda en `layout.side`; `sections` y `sections_width` salieron de las
  preferencias (los archivos antiguos siguen cargando: se ignoran claves desconocidas).
- Verificado: Clippy, 23 pruebas unitarias y captura del arranque real con la vista
  restaurada desde las preferencias. Sin QA de ratón en esta vista ni de macOS/Linux.

## UX del Studio: menú, barra de herramientas y búsqueda (2026-11-13, Windows)

- Reporte del usuario: el rail dejaba varias marcas activas a la vez y la lupa no
  mostraba nada sin un archivo abierto (solo buscaba dentro del archivo). Se rehízo:
  rail con selección única por grupo (`shell::Side` y `DockTab`), búsqueda real de
  proyecto (`features/project_search.rs`, `studio_search.rs`), barra de menú
  (`studio_menu*.rs`) y barra de herramientas (`studio_toolbar.rs`) con estados
  desactivados y tooltips. Atajos globales nuevos: `Ctrl+Shift+F`, `Ctrl+S`, `Ctrl+W`,
  `Esc`. Los resultados abren el archivo en la línea y seleccionan la coincidencia.
- Hallazgo de render: la sombra difuminada de un panel que se superpone a otros widgets
  la pinta el renderer por software ENCIMA del propio panel. El menú usa un estilo sin
  sombra (`styles::menu_panel`); el diálogo modal conserva su sombra porque cubre toda
  la ventana. No añadir sombras a overlays parciales sin comprobarlo en pantalla.
- Verificado: 32 pruebas (23 unitarias, 6 de catálogo, 3 de producto), Clippy, formato y capturas del arranque real a 880
  px, del menú Proyecto abierto, de la búsqueda con resultados y de abrir en una línea.
  No verificado: clics reales de ratón en menú y barra (se probó la lógica por pruebas
  unitarias y el estado forzado en un build temporal), tooltips y navegación por teclado
  dentro de los menús (flechas), macOS/Linux.

## Rendimiento, catálogo y persistencia del Studio Rust (2026-11-13, Windows)

- Rendimiento medido (120 redibujados, mismo equipo): build debug anterior 18,6 s de CPU
  y arranque de 2,1 s; release software 1,2 s de CPU, 117 ms y 49 MB; release con GPU
  (wgpu) 3,75 s de CPU y 203 MB. El «pegado» era sobre todo el build debug. Decisión:
  renderer por software, sin wgpu; `[profile.dev.package."*"] opt-level = 3` y release
  con LTO. Además, arranque, previews, doctor, proyectos, rollback y trust salen del
  hilo de UI (`studio_jobs`/`studio_plans`) y la terminal solo reconstruye su texto si
  llegó salida (`poll_changed`).
- Catálogo portado (R3.3 parcial): `catalog`, `catalog_view`, `project_skills`,
  `project_mods`, `project_mod_agents` y `project_changes` (transacción compartida:
  snapshot, backup, escritura atómica, rollback). Datos del legado (21 skills, 13
  packs, 23 mods) embebidos por `build.rs` desde `src/chxchx_tech_lead/`. CLI
  `skills|packs|mods` y página «Catálogo» en Studio con vista previa y confirmación.
- Persistencia: `features/studio_preferences.rs` + `studio_session.rs`; ver README de
  `migration/rust`. Verificado reabriendo la app: restaura ventana, paneles, página,
  pestañas, pestaña activa y carpetas, y descarta archivos desaparecidos.
- Verificado en Windows: formato, Clippy all-targets con warnings denegados, 25 pruebas
  (16 unitarias, 6 de catálogo, 3 de producto) y checks por feature (cli, tui, studio).
  Se corrigió una prueba intermitente previa (`project_files_tests`: carpetas temporales
  con el mismo nombre por la resolución del reloj en Windows).
- No verificado: clics reales en la página Catálogo (aplicar/retirar desde la GUI; sí
  lo cubren las pruebas CLI del mismo caso de uso), atajos y arrastre de separadores
  con ratón, TUI del catálogo (no existe), macOS/Linux y CI remoto.

## Estado actual

- Producto: control plane local con CLI/TUI en Python y ChxChx Studio en C++20/Qt 6 Widgets con Scintilla/Lexilla. Studio llama al CLI mediante el bridge JSON v1 (`workspace/bridge_contracts.py`); Python sigue siendo la autoridad. La rama de integración es `dev`.
- Studio cubre todas las áreas de la TUI (inventario en `.ai/EDITOR-PARITY.md`). Toda escritura pasa por `--dry-run`, preview y confirmación. Inicio ofrece `Preparar y entrar`. Incluye terminal PTY/ConPTY + libvterm (scrollback de 5000 líneas), pestañas de agentes Codex/Claude/OpenCode (+Ollama), búsqueda de rutas y contenido, archivos recientes y grupos divididos, diff Git por archivo, símbolos básicos, atajos configurables y paleta `Ctrl+P`.
- Catálogo: 21 skills, 13 Tech Packs y 9 Mods de Claude Code en `.claude/skills`. RAM Governor: `max_agents` 2, aviso RAM 70 %, crítico 85 % y aviso swap 40 %; trust local obligatorio antes de ejecutar procesos.
- CI/CD de proyectos del usuario: CI opt-in para Python/Node/CMake y CD opt-in a VPS con Docker Compose (ADR 0007), ambos en CLI/TUI/Studio.
- Distribución: CPack genera ZIP/DMG/TGZ y un `Setup.exe` NSIS. Stable `0.5.0` se instaló y verificó por versión en Fedora x86_64; no se lanzó la ventana. No se ha probado la instalación en equipos limpios ni en Windows/macOS.
- Memoria: `.ai/memory/PROJECT_MEMORY.md` guarda las peticiones explícitas, y Basic Memory se usa limitado al `memory_project` del proyecto. Al cerrar cada sesión de Codex/Claude se escribe un resumen en `.ai/history/SESSIONS.md`, que se inyecta en el siguiente chat nuevo. `CURRENT_STATE`/`HANDOFF` se compactan solos hacia `.ai/history/`.
- Lo que bloquea el uso diario: QA interactivo de Studio en Fedora con un proyecto real. El orden de trabajo está en `.ai/ROADMAP.md` (plan de noviembre 2026).
- Equipo actual: Fedora 44 con CMake 4.3, GCC 16, Qt 6.11 y Ninja. Studio stable `0.5.0` está en `~/.local/opt/chxchx-studio/stable-0.5.0`; el launcher `~/.local/bin/chxchx-studio` y la entrada de menú están instalados. La antigua instalación compilada de `dev` (`5eae006`) y sus accesos se retiraron el 2026-11-04. El estado anterior completo está en `.ai/history/CURRENT_STATE.md`.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.
