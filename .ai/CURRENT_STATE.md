# Current State

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

## Rediseño visual IDE con marca ChxChx en Studio Rust (2026-11-13, Windows)

- Studio Rust (Iced 0.14, renderer software) tiene nueva capa visual tipo IDE:
  barra de título con isotipo y proyecto, rail de actividad con indicador y
  tooltips, explorador con chevrons e iconos por tipo de archivo, pestañas con
  marcador activo, breadcrumbs con acciones del editor, panel inferior con
  pestañas Terminal/Actividad, barra de estado con segmento de marca, cursor y
  lenguaje, y diálogo modal para descartar/salir. Inicio muestra hero con el
  logo, tarjeta de proyecto, siguiente paso y chips de tecnologías.
- Marca: `migration/rust/assets/brand/` (isotipo 40/256 px, logo 420 px y
  `.ico`) generado desde `assets/` de la raíz. Icono de ventana en runtime y
  recurso incrustado en el `.exe` por `build.rs` (embed-resource; solo Windows
  con feature `studio`, no bloquea la compilación si falla).
- Paneles redimensionables con separadores arrastrables (`studio_shell.rs`) y
  atajos `Ctrl+B` / `Ctrl+J`. Dock inferior con pestañas excluyentes.
- Módulos: tokens en `studio_design`, estilos en `studio_styles`, marca en
  `studio_brand`, iconos SVG cacheados y teñidos en `studio_icons`, además de
  `studio_file_types`, `studio_home`, `studio_ide_tabs`, `studio_ide_dock` y
  `studio_shell`. Todos bajo 300 líneas. Sin cambios en casos de uso ni adapters.
- Dependencias nuevas: `image` (solo PNG) vía `iced/image-without-codecs` y
  `embed-resource` como build-dependency. Fuentes del sistema, sin incrustar.
- Verificado en Windows: formato, Clippy all-targets con warnings denegados,
  12 pruebas Rust (9 unitarias, 3 CLI), check CLI-only y build de Studio.
  Revisión visual por captura de: inicio, editor, dock + búsqueda + aviso,
  diálogo modal y página Proyectos. No se probó con ratón real el arrastre de
  separadores ni los atajos (solo la prueba unitaria del estado), ni el panel de
  revisión con datos, ni la terminal con sesión activa. Sin QA en macOS/Linux.
- Basic Memory no está expuesto; checkpoint local.

## Port de la estructura IDE Qt a Rust (2026-11-13, Windows)

- Dirección aclarada: recuperar la misma cara del IDE Qt, no un dashboard como
  producto final. Shell Medianoche azul/cian con explorador, pestañas centrales,
  secciones a la derecha y paneles inferiores de terminal/actividad.
- Editor Iced funcional: archivos UTF-8 acotados, resaltado, búsqueda,
  deshacer/rehacer, dirty state, confirmación al descartar, guardado atómico con
  respaldo externo y detección de conflictos. Casos de uso en `project_files`;
  UI en `studio_editor*` e `studio_ide*`, sin módulos sobre 300 líneas.
- Capa visual del IDE ampliada: iconografía vectorial SVG inspirada en
  `native/src/studio_icons.cpp`, cabecera de workspace, barra de accesos rápidos,
  rail de actividad, iconos en árbol/pestañas/secciones/tarjetas, panel inferior
  Terminal/Actividad y barra de estado. El editor conserva el tema Medianoche y
  el foco usa un borde cian tenue.
- Terminal Rust con portable-pty/vt100, trust local obligatorio, entrada por
  líneas, Ctrl+C, resize y cierre local. Smoke Windows comprobó salida, entrada
  PowerShell y salida del proceso. Se resolvió handshake DSR de ConPTY que
  bloqueaba el arranque; lector/respuesta se inician antes del shell.
- Clippy all-targets y 11 pruebas Rust pasan (8 unitarias y 3 CLI). La rama de
  prueba de symlink solo corre si el SO permite crearlo. Build/QA de otros SO
  pendiente. R1/R4/R6 siguen abiertos: terminal interactiva completa, Git,
  agentes/chat, paneles acoplables, edición avanzada y accesibilidad no portados.
- Arranque visual Windows revisado con archivo Rust abierto; iconos y barras
  nuevos revisados en pantalla con `studio_ide_chrome.rs` en el editor.
  Check CLI-only, compileall y 2 tests de workflows también pasan. No se repitió
  la suite Python completa: el cambio funcional está en Rust y su CI.
- Basic Memory no está expuesto; checkpoint local.

## Rediseño del Studio Rust (2026-11-13, Windows)

- Shell Iced con navegación Inicio/Preparación/Proyectos/Integraciones/Diagnóstico,
  tema oscuro y acento verde, resumen del proyecto, siguiente paso contextual,
  tarjetas de acciones y revisión antes de confirmar. Ventana inicial 1120×500
  con contenido desplazable para el escritorio Windows disponible.
- Se extrajo la presentación a `studio_view`, `studio_pages`, `studio_review` y
  `studio_design`; los handlers existentes viven en `studio_actions`. Todos los
  módulos quedan por debajo de 300 líneas. No cambia la autoridad del dominio.
- Formato, check, Clippy con warnings denegados y tests CLI (3 passed) pasan.
  Build/arranque Windows y primera revisión visual realizados. Falta QA manual
  de todas las páginas, teclado, escalado y accesibilidad; R1 sigue abierto.
- Basic Memory no está expuesto; checkpoint conservado en archivos locales.

## Continuación activa: Studio nativo Rust en Windows (2026-11-13)

- El usuario confirmó que el objetivo es una GUI nativa Rust y pidió pausar
  Fedora hasta poder revisar allí el resultado. El Studio Rust actual usa Iced;
  Qt/C++ pertenece al Studio legado.
- Windows x86_64 MSVC: `cargo build --locked --manifest-path
  migration/rust/Cargo.toml --bin chxchx-studio` pasó. Se abrió el ejecutable
  desde la raíz y el proceso respondió en Windows. Esto solo valida build y
  arranque del dashboard existente; R1 sigue abierto para editor, terminal,
  IME, accesibilidad, evaluación del toolkit y otras plataformas.
- Formato Rust, `cargo check` full/CLI, Clippy, build Studio offline, tests CLI
  (3 passed), schema v1 y `python -m compileall -q src` pasaron. Suite Python:
  475 passed, 37 skipped, 10 deselected. La prueba de symlink ahora omite solo
  el error Windows 1314 al crear el fixture; si el enlace se puede crear, la
  comprobación de seguridad se ejecuta normalmente.
- Se cerraron 120 procesos E2E de OpenCode falsos hallados en TEMP tras las
  corridas; el filtro exigió el comando `opencode-fake.py` y no quedan activos.
- Se interrumpió por solicitud del usuario la compilación Fedora 44 del Studio
  legado en 219/386 tareas. `podman stop` terminó el contenedor (código 137);
  no se alcanzaron CTest, CPack ni el verificador. Después se eliminó la VM
  `podman-machine-default` y su disco/imágenes; el CLI Podman sigue instalado.
  R0.4 sigue abierto y diferido hasta ejecutar la validación en Fedora.
- La carpeta temporal del build parcial (443 MB) sigue en `$TEMP`: la revisión
  automática rechazó el borrado recursivo. Se puede localizar por el prefijo
  `chxchx-r04-fedora44-*`.
- La configuración candidata de deploy CPack Fedora sigue sin comprobarse.
  Cuando se retome, separar ese paquete legado de la distribución Rust.
- Basic Memory/Serena y `.ai/SKILLS.md` no están disponibles en este checkout;
  el checkpoint está en `.ai/HANDOFF.md`.

## R2.4: pruebas iniciales y fixes hallados en Windows (2026-11-13)

- Añadida suite CLI en `migration/rust/tests/product_workflows.rs`: `init
  --minimal` verifica preview sin escrituras, apply e idempotencia; trust
  verifica preview/apply/untrust y backup; `projects list` verifica registro
  ausente y JSON inválido preservado. Cada caso usa home/PATH/proyecto temporal.
- Los tests encontraron dos fallos reales: la validación de directorios de init
  comparaba una ruta canónica Windows `\\?\` con la raíz normalizada, y el
  validador Rust rechazaba `header.logo = ""` aunque el modelo Python lo admite
  y la plantilla predeterminada lo genera. Se normaliza la ruta canónica y se
  conserva la regla de tipo texto sin exigir contenido para el logo.
- El workflow `rust-product.yml` ejecuta las pruebas CLI y valida la salida JSON
  real de `status --json` contra Draft 2020-12 usando `jsonschema==4.26.0`.
  La implementación de R2.4 está lista, pero el gate sigue abierto hasta que
  la CI remota confirme los tres runners; la evidencia local es Windows. R3.4
  aún requiere paridad amplia, previews obsoletos, rollback/fallos y preservación
  de datos ajenos.
- Checks Windows: formato Rust, `cargo test --locked --offline
  --no-default-features --features cli` (3 passed) y
  `uv run --with-requirements scripts/requirements-rust-schema.txt python
  scripts/validate_project_overview_schema.py` pasan.
- R0.4 y R1 siguen abiertos; R2.1 conserva incompleta la estructura `app/`,
  `runtime/` y `protocol/`. Próximo gate de producto: R0.4 en Fedora, después
  viabilidad R1 y avance por capacidades R3.

## Revisión para retomar en Windows (2026-11-13)

Este checkpoint prevalece sobre los siguientes para el orden de trabajo. Los
resultados Fedora anteriores siguen siendo evidencia de ese entorno, no de este
checkout Windows.

- Rama `migracion-rust`, HEAD revisado `619edfa`; árbol limpio al
  iniciar la revisión. Versiones locales: legado `0.5.4`, paquete Rust
  `0.6.0-dev.0` (no publicado). Python/Qt conservan la autoridad de producto.
- Confirmado en código: Rust comparte overview/detección, init, trust,
  backups/rollback, MCP/Basic Memory, doctor y `projects list/current/sync`
  entre CLI/TUI/Studio. Studio sigue siendo un dashboard Iced sin editor ni
  terminal; `projects switch` y el runtime de procesos no están portados.
- R0.1–R0.3 tienen evidencia documentada; R0.4 sigue parcial por el TGZ Fedora
  incompleto. Falta generar/verificar un paquete válido, medir su tamaño y
  cerrar los presupuestos; tampoco hay medición de un escenario activo.
- R1 sigue abierto, incluido el ADR del toolkit/editor. R2.1–R2.3 figuran
  completos en ROADMAP, pero la estructura actual no tiene todavía módulos
  `app/`, `runtime/` ni `protocol/`; R2.1 no debe interpretarse como runtime
  implementado. R2.4 sigue abierto: el workflow solo hace formato/check/Clippy,
  sin pruebas de capacidades ni validación del schema JSON.
- No se encontró suite Rust (`tests/`, `#[test]` o `#[cfg(test)]`). Las
  mutaciones existentes necesitan cobertura de preview, idempotencia, planes
  obsoletos, preservación de datos y rollback antes de ampliar sus garantías.
- Orden de reanudación: cerrar R0.4 en Fedora; validar prototipo R1; completar
  R2.4 con contratos y pruebas de capacidades existentes; después continuar
  R3–R7. La cobertura existente puede adelantarse en Windows sin esperar a
  `projects switch` ni a los instaladores.
- Verificación Windows: `python -m compileall -q src` pasó (Python 3.13.0).
  `uv run pytest` usó Python 3.12.14: 475 passed, 36 skipped, 10 deselected,
  1 failed. El fallo es `test_a_symlinked_context_file_is_not_followed`:
  `WinError 1314` al crear el symlink del fixture, antes de invocar el producto.
  Sigue pendiente resolver la portabilidad de esa prueba o validarla en un
  runner con permiso de symlinks; la suite local no queda completamente verde.
- Rust: formato, `cargo check --locked --offline` completo y CLI sin GUI,
  y `cargo clippy --locked --offline -- -D warnings` pasaron (Cargo 1.99.1).
  No se ejecutaron build Qt, pruebas Rust, QA visual ni CI remoto.
- Basic Memory/Serena no están expuestos entre las herramientas disponibles;
  el checkpoint queda en estos archivos. No se modificó código del producto.

## Baseline de rendimiento legado, R0.4 parcial (2026-11-12)

- Medición Release en Fedora 44: `.ai/PERFORMANCE-BASELINE.md` contiene
  fixtures, hardware, comandos, latencia p50/p95, RSS, CPU y tamaños.
- En proyecto grande (5,001 archivos): CLI `status` p95 372 ms; proxy de
  arranque Studio p95 330 ms; RSS idle máximo 67.1 MiB; índice p95 172 ms;
  búsqueda de rutas p95 9.18 ms/100 llamadas; contenido p95 136 ms.
- El TGZ CPack mide 4.31 MB pero falla el verificador por faltar bibliotecas
  Qt/plugins. No fijar presupuesto de paquete ni cerrar R0.4 hasta corregirlo.
- Añadidos generador de fixtures y harness opt-in para benchmark de índice;
  no se cambió funcionalidad del producto. No se ejecutaron tests en este tramo.

## Baseline legado en Fedora 44 (2026-11-12)

- Python: `python -m compileall -q src tests scripts` pasó en Python 3.14.10;
  `uv run pytest --junitxml=/tmp/chxchx-rust-migration-baseline-pytest.xml`
  pasó 511 tests, 1 skipped y 10 deselected con Python 3.11.18 de uv.
- Studio legado: configuración y build CMake/Ninja pasaron con GCC 16.2.1,
  CMake 4.3.0, Ninja 1.13.2 y Qt 6.11.2. CTest pasó 11/11 smoke, incluidos
  Git UI y terminales. Build aislado en `/tmp`; warnings deprecados solo en
  Scintilla/Qt vendorizados.
- El Rust product ya compila aquí con `cargo check --locked` (ver sección de
  matriz). R0.3 queda reproducido en Fedora; fuera de evidencia quedan live,
  empaquetado/release, QA visual y otras plataformas. R0.4 aún requiere medir.
- No se modificó código del producto. Basic Memory/Serena no están disponibles.

## Matriz de capacidades y datos Rust (2026-11-12)

- Creada `.ai/RUST-CAPABILITY-MATRIX.md` con dueño/evidencia por capacidad,
  estado Rust, registros/rutas globales y por proyecto, y todas las claves
  QSettings localizadas en el Studio legado. R0.2 queda documentado; R0.3/R0.4
  y R1 siguen abiertos.
- `.gitignore` permite versionar la matriz. En Fedora 44, `rustc/cargo 1.100.0`
  y `cargo check --manifest-path migration/rust/Cargo.toml --locked` pasan.
  No se ejecutaron suites ni cambió código. Basic Memory/Serena no están
  disponibles.

## Revisión de brechas de migración Rust (2026-11-12)

- El paquete unificado en `migration/rust/` ya comparte catálogo, preparación,
  trust, MCP, sync y doctor entre CLI/TUI/Studio. La paridad global sigue abierta.
- El roadmap referencia `.ai/RUST-CAPABILITY-MATRIX.md`, pero ese archivo no
  existe en el checkout. R0.2–R0.4 y R1 siguen sin cerrar; la CI compila y corre
  Clippy, pero aún no valida schemas ni ejecuta pruebas Rust.
- Antes de ampliar runtime/switch, reconstruir la matriz y evidencia del legado,
  habilitar contratos y pruebas de capacidades en CI, y cerrar los gates R0/R1.
  Después validar diseño del supervisor/runtime y abordar `projects switch`.
- Sin cambios de código en esta revisión. Basic Memory y Serena no están
  disponibles en la sesión; el checkpoint se conserva localmente.

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Sincronización de proyectos Rust (2026-11-12)

- `features/project_sync.rs` previsualiza y aplica `projects sync` sobre el
  catálogo local. Actualiza solo los bloques administrados `project-rules` de
  `AGENTS.md` y `claude-rules` de `CLAUDE.md`; conserva el texto manual restante.
- La vista previa guarda snapshots del registro y los archivos. Apply rechaza
  cambios obsoletos, crea backup por proyecto modificado y revierte los archivos
  del proyecto si una escritura falla. Proyectos duplicados, ausentes, ilegibles
  o con destinos enlazados se omiten con aviso; otros proyectos pueden continuar.
- CLI: `projects sync` previsualiza y `--apply` aplica. TUI: `s`, revisar, `y`;
  Studio muestra el plan y aplica desde worker. Los tres usan el mismo caso de uso.
- Validación Windows: `cargo check --locked` default y CLI/TUI/Studio por
  separado, Clippy con warnings denegados, formato y `git diff --check` pasan.
  No se ejecutaron pruebas ni se modificaron proyectos registrados.
- Pendiente: `projects switch` junto al runtime workspace/agentes e instaladores.
- Basic Memory/Serena no exponen `write_memory`; checkpoint local en `.ai/`.

## Catálogo de proyectos Rust (2026-11-12)

- `features/project_catalog.rs` lee `projects.json` sin modificarlo y comparte
  los DTO de `projects list/current` entre CLI, TUI y Studio. La lista muestra
  alias, nombre, perfil, stacks, ruta, disponibilidad y último proyecto.
- Registro ausente se interpreta como catálogo vacío; JSON inválido o `projects`
  con tipo incorrecto produce error visible, y un `projects.json` enlazado no se
  lee. Entradas inválidas/no-objeto se omiten como en el lector Python.
- CLI: `projects list` y `projects current`; TUI: `p`; Studio: botón de proyectos.
  `switch` depende del runtime workspace/agentes aún no portado. `sync` se
  describe en la sección anterior.
- Validación Windows: `cargo check --locked` default y CLI/TUI/Studio por
  separado, Clippy con warnings denegados, `cargo fmt --all -- --check` y
  `git diff --check` pasan. No se ejecutaron pruebas ni CLIs externos.
- No se ejecutó la suite ni comandos externos; Basic Memory/Serena no exponen
  `write_memory`, así que este checkpoint es local.

## Basic Memory durante init en Rust (2026-11-12)

- `integrations/basic_memory.rs` concentra la detección del CLI y el registro
  idempotente de `.ai/memory`; `init` completo y compacto lo invocan desde el
  caso de uso compartido para CLI, TUI y Studio.
- La vista previa solo busca el ejecutable en `PATH`; no invoca Basic Memory.
  `apply` lo ejecuta después de completar archivos, directorios y registro local.
  Si falta el CLI o el registro falla, init conserva la preparación local y
  devuelve un aviso; el resultado distingue proyecto registrado, preexistente
  y fallo.
- TUI y Studio ejecutan el apply completo en segundo plano y bloquean otras
  mutaciones hasta recibir el resultado; CLI conserva el flujo secuencial.
- `integrate --apply` reutiliza el mismo adapter. El catálogo ya expone
  `projects list/current` y `projects sync`; `switch` e instaladores siguen pendientes.
  No se ejecutaron comandos Basic Memory.
- Validación Windows: `cargo check --locked` default y CLI/TUI/Studio por
  separado, `cargo clippy --locked -- -D warnings`, `cargo fmt --all -- --check`
  y `git diff --check` pasan. No se ejecutaron tests, Basic Memory ni clientes
  MCP.
- Basic Memory/Serena no exponen `write_memory` en esta sesión; checkpoint local.

## Integraciones MCP y ejemplo OpenCode en Rust (2026-11-12)

- `features/mcp_setup` comparte preview y aplicación entre CLI, TUI y Studio.
  `integrate [PATH] --client all|claude|codex|opencode` no escribe por defecto;
  `--apply` habilita el CLI oficial de cada cliente y el registro del proyecto
  en Basic Memory. Codex usa `.codex/config.toml` del proyecto; `--refresh`
  reemplaza solo las entradas MCP conocidas después de mostrar el plan.
- TUI presenta el plan con `c` y aplica con `y`; Studio ofrece previsualizar y
  aplicar desde un worker. Ambas usan el mismo caso de uso que CLI.
- El init completo conserva el ejemplo `.ai/integrations/opencode-mcp.example.json`
  en el plan con backup/rollback. Crear el ejemplo no inicia servidores.
- Adapter de procesos con timeout de 30 s y captura limitada a 256 KiB; en
  Windows resuelve wrappers `.ps1`, `.cmd` y `.bat` pasando argumentos como JSON,
  sin interpolarlos en el código PowerShell. Configuración Codex TOML inválida,
  enlaces `.codex` y previews obsoletos bloquean escrituras.
- El formato Codex está aislado en `integrations/mcp_codex.rs`; resolución de
  ejecutables y transporte de comandos viven en `integrations/mcp_clients.rs`.
- `features/doctor` arma un reporte común para CLI (`doctor`), TUI (`d`) y Studio.
  Comprueba presencia en PATH, archivos requeridos, configuración/trust, cwd de
  procesos/agentes y configuración MCP local/global de Claude/Codex sin lanzar
  agentes, servidores ni órdenes configuradas.
- Verificación local Windows: formato, `cargo check --locked` default y por
  feature CLI/TUI/Studio, Clippy con warnings denegados y `git diff --check`.
  No se ejecutaron pruebas, CLIs de terceros ni CI macOS/Linux.
- Basic Memory/Serena no exponen `write_memory` en esta sesión; checkpoint
  conservado en `.ai/CURRENT_STATE.md`, `.ai/HANDOFF.md` y roadmap local.
- Pendiente: validar la integración MCP con versiones instaladas de los
  clientes; los instaladores y la navegación de proyectos siguen por migrar.

## Preparación y rollback de proyecto en Rust (2026-11-12)

- `features/project_setup` comparte init completo y compacto en CLI/TUI/Studio.
  CLI muestra preview y requiere `--apply`; TUI usa `i`/`m` y `y`; Studio tiene
  botones para previsualizar y aplicar la misma operación.
- El modo completo crea solo directorios/plantillas faltantes, actualiza los
  bloques administrados sin borrar texto manual, crea config v2 o migra v1→v2,
  preserva campos TOML desconocidos y actualiza `projects.json` manteniendo el
  resto del registro. Configuración local y registro se respaldan antes de
  reemplazarse; ningún comando sugerido se ejecuta.
- El modo `--minimal` crea config/memoria faltante y actualiza el registro. Las
  rutas enlazadas o externas, formatos inválidos y cambios posteriores a preview
  bloquean la aplicación. Fallos locales disparan rollback de archivos; el
  registro también se restaura si falla su escritura.
- `rollback [PATH]` selecciona el último backup del proyecto y comparte preview
  y aplicación (`--apply` en CLI, `b`/`x` en TUI y botones en Studio). Captura
  huellas para rechazar previews obsoletos, crea un backup de seguridad antes
  de restaurar y revierte automáticamente si la restauración falla.
- `workspace trust/untrust [PATH]` usa el mismo preview/apply. El trust se ata
  al SHA-256 de la configuración actual (también permite confiar un proyecto
  sin config), guarda backup del registro global y se invalida al cambiar TOML.
  TUI: `t`/`u` y `y`; Studio ofrece preview y guardado. Ninguna de estas
  operaciones inicia procesos.
- La integración MCP está descrita en la sección superior. El resto de la
  preparación de init no inicia herramientas externas.

## Port de detección y perfiles a Rust (2026-11-12)

- `project-overview` ahora incluye stacks, lenguajes, gestores, infraestructura,
  perfil y reglas; el CLI, la TUI y Studio consumen el mismo resultado.
- Portado el recorrido del detector: omite carpetas de dependencias/build, lee
  marcadores JS/Python/.NET/C++/Qt/Postgres/Redis/Docker/AI y carga perfiles
  integrados con prioridad, más perfiles locales de `$CHXCHX_TECH_HOME`.
- La lectura de config ahora distingue parseo, versión y compatibilidad TOML v1/v2;
  comprueba estructura de workspace, límites de recursos, comandos, IDs,
  presets y rutas cwd dentro de la raíz. Solo lectura: no ejecuta comandos,
  migra ni escribe la configuración existente.
- Añadido el catálogo de perfiles como asset del paquete Rust y ampliado
  `chxchx.project-overview` v1 mientras sigue interno/no publicado. Este port no
  modifica el comportamiento Python/Qt ni sustituye el bridge de status.
- `cargo fmt`, `cargo check --locked`, builds CLI-only/TUI-only y `cargo clippy
  -- -D warnings` pasan en Windows; CI ahora incluye Clippy en la matriz de SO.
  No se agregaron ni ejecutaron tests por
  instrucción de la sesión; falta evidencia de paridad funcional y build/QA en
  macOS/Linux.

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
