# Handoff

## Sugerencias de commit (2026-11-13)

- Reglas en `suggest_rules.rs` (clasificación, nombres, tipo), `suggest_split.rs`
  (módulos grandes), `suggest_checks.rs` (secretos, formato). Para ajustar una regla,
  añadir un caso en `suggest_tests.rs`. Cuando existan agentes (R4) se puede ofrecer una
  sugerencia con IA además de esta, con consentimiento y sin enviar archivos sensibles.

## Git (2026-11-13)

- Pedido: Git completo en la barra izquierda, como el Studio Qt, con recomendaciones.
  Hecho salvo lo listado como «No portado» en `.ai/CURRENT_STATE.md`.
- Las pruebas de `features/git/tests.rs` crean repositorios temporales con el `git` real y
  se omiten si no existe. Usan `core.autocrlf=false` para ser deterministas en Windows.
- Siguiente paso natural: el asistente (sugerir mensaje / consejo) cuando se porten los
  agentes (R4); hoy la guía es determinista y local.
- Un repositorio con el proyecto en una subcarpeta usa `Repository::prefix`: los archivos
  fuera de la carpeta no se muestran en el explorador y no se pueden abrir desde el panel.

## Espacio de trabajo a la izquierda (2026-11-13)

- Pedido del usuario: mover «Espacio de trabajo» del lado derecho al menú izquierdo para
  ganar pantalla. Hecho; detalle en `.ai/CURRENT_STATE.md`. Para añadir una sección,
  una entrada en `GROUPS` de `studio_workspace.rs`, la página en `Page` y su menú/atajo.

## UX del Studio (2026-11-13, Windows)

- Pedido: que no se sienta bugueado ni pegado, con barra de menú y de herramientas.
  Detalle y hallazgos en `.ai/CURRENT_STATE.md`.
- El rail ya no contiene páginas; navegar secciones es por menú Ir, barra de
  herramientas y panel derecho. Para añadir un comando: una entrada en
  `studio_menu_items.rs` y, si es frecuente, un botón en `studio_toolbar.rs`; los que
  inician un plan se encadenan con `Message::Sequence` para abrir antes su sección.
- Pendiente: navegación con flechas dentro de los menús, buscar y reemplazar,
  búsqueda con regex/mayúsculas, numeración de líneas, y QA de ratón real.
- Estado del código del Studio: todos los módulos bajo 300 líneas.

## Rendimiento, catálogo y persistencia (2026-11-13, Windows)

- Pedido del usuario: Studio Rust lento/pegado, portar packs, plugins, skills y demás,
  y dejar la persistencia completa sin perder el diseño. Detalle y mediciones en
  `.ai/CURRENT_STATE.md`.
- «Plugins» se interpretó como los Mods del legado (instrucciones para agentes). No se
  portó el catálogo de agentes (`agent_catalog.py`), los modelos locales (Ollama), Git,
  chat ni CI/CD: siguen en el legado y son el siguiente bloque de paridad (R4/R6).
- El catálogo se lee de `src/chxchx_tech_lead/` al compilar (`build.rs`, constante
  `LEGACY`). Al promover Rust a la raíz hay que mover esos directorios y cambiar la
  constante; hasta entonces hay una sola fuente de verdad, sin copias que se desfasen.
- Pendiente de QA manual en la GUI: aplicar/retirar skills y mods con clics reales,
  arrastre de separadores, atajos y escalado. La TUI no tiene catálogo (R3.4).
- Excepciones a las 300 líneas: `interfaces/cli.rs` (538, ya excedía antes; siguiente
  paso: extraer los comandos de proyecto a `cli_projects.rs`).
- Para medir rendimiento usar `--release`; el script de benchmark usó 120 cambios de
  tamaño de la ventana y el CPU del proceso. No habilitar wgpu sin repetirlo.

## Rediseño visual IDE con marca ChxChx (2026-11-13, Windows)

- El usuario pidió remodelar el diseño del Studio Rust: aspecto de IDE
  profesional, marca ChxChx con los logos de `assets/` y solo tecnología nativa
  de Rust. Hecho sobre Iced existente; detalle en `.ai/CURRENT_STATE.md`.
- Para cambiar colores o tipografía editar `studio_design.rs`; para el aspecto
  de un widget, `studio_styles.rs`. Si cambia el logo de `assets/`, regenerar
  `migration/rust/assets/brand/` (recorte al contenido y reescalado con Pillow:
  isotipo 40 y 256 px, logo 420 px de alto, `.ico` multitamaño).
- Pendiente de QA manual: arrastre de separadores, `Ctrl+B`/`Ctrl+J`, panel de
  revisión con planes reales, terminal con sesión activa, escalado HiDPI y
  teclado/accesibilidad. Pendiente de CI: build Linux/macOS con `image` y la
  build-dependency `embed-resource` (en esos SO el script no hace nada).
- Ideas no implementadas: barra de título propia sin decoraciones nativas,
  numeración de líneas (el `text_editor` de Iced no trae gutter), tema claro,
  fuentes incrustadas y persistencia del tamaño de paneles.
- Sigue vigente: R1/R4/R6 abiertos; no presentar este port como IDE completo.
- Para capturas de verificación usar `PrintWindow` sobre el handle de Studio;
  no capturar la pantalla ni enviar teclas globales.

## Port IDE activo: misma estructura del Qt en Rust (2026-11-13)

- Usuario pide IDE completo y misma cara del Qt. Se sustituyó el shell de
  dashboard por disposición Qt: explorador izquierdo, pestañas/editor central,
  secciones derechas y panel inferior. Paleta Medianoche tomada de
  `native/src/studio_themes.cpp`. Las páginas operativas viven dentro de Inicio.
- Esta pasada añade iconos SVG basados en el set Qt, cabecera con workspace,
  toolbar de accesos rápidos, navegación vertical de actividad, iconos en árbol,
  pestañas, secciones y tarjetas, selector de panel inferior y barra de estado.
  Se revisó la app corriendo en Windows; la ventana queda abierta mostrando un
  archivo Rust.
- `project_files` implementa listado lazy, lectura UTF-8 <=2 MiB, guardado con
  respaldo en home/editor-backups, conflictos y rechazo de enlaces/rutas externas.
  Editor con resaltado, búsqueda, undo/redo, tabs y confirmación de descarte.
  Bin Studio acepta raíz y archivo relativo opcional; `run(root)` se conserva.
- `integrations/terminal` encapsula portable-pty/VT; `workspace_terminal` valida
  trust. UI de líneas + Ctrl+C, resize y panel ocultable; no representa todavía
  una terminal interactiva completa. Se corrigió el handshake ConPTY de cursor.
- Checks Windows: Clippy all-targets y 11 tests Rust (8+3) pasan. CI ahora prueba
  también las capacidades terminal con default features. Falta evidencia remota.
- La UI visual adicional pasó `cargo check --locked --bin chxchx-studio` y
  `cargo build --locked --bin chxchx-studio`; no se repitieron pruebas.
- Se revisó visualmente el ejecutable Windows con archivo Rust abierto. Check
  CLI-only, compileall y 2 tests de workflows pasan. Terminal resize y send
  corren en workers; no se concedió trust al repo para la demostración visual.
- Siguiente paridad: terminal con teclado raw/colores/scrollback, Git visual,
  agentes/chat/permisos, editor avanzado y paneles acoplables. R1/R4/R6 siguen
  abiertos; no presentar este port como IDE completo ni retirar Qt/Python.
- Módulos nuevos bajo 300 líneas; Basic Memory no disponible, checkpoint local.

## Rediseño activo de Studio Rust (2026-11-13, Windows)

- Implementado shell Iced con cinco áreas operativas, estado del proyecto,
  recomendación contextual y tarjetas. Acciones existentes conservan sus planes
  y confirmación en un panel de revisión. Tema oscuro/verde y texto orientado
  a preparar, revisar permisos, conectar y recuperar respaldos.
- Presentación separada en módulos `studio_*`, todos menores de 300 líneas;
  sin cambios a casos de uso ni integraciones. Ventana 1120×500 y scroll.
- Check/Clippy/formato y 3 tests CLI pasan. Build y arranque Windows comprobados;
  primera captura revisada y tamaño inicial corregido para caber en pantalla.
- Siguiente: QA interactivo de navegación, previews, teclado y escalado; después
  avanzar prototipo editor/terminal de R1. Este rediseño cubre las capacidades
  portadas; todavía no acredita paridad completa ni accesibilidad.
- Basic Memory no está disponible; checkpoint local.

## Continuación activa: Studio nativo Rust en Windows (2026-11-13)

- El usuario confirmó que el destino es Rust nativo y pidió pausar la validación
  Fedora para comprobar también Windows. El Studio Rust actual usa Iced; Qt/C++
  corresponde al legado, no a la interfaz nueva.
- Windows x86_64 MSVC: `cargo build --locked --manifest-path
  migration/rust/Cargo.toml --bin chxchx-studio` pasó. Se abrió
  `migration/rust/target/debug/chxchx-studio.exe` desde la raíz y Windows
  informó el proceso `chxchx-studio` como responsivo. Esta evidencia solo cubre
  build y arranque del dashboard actual; no cierra el prototipo R1 de editor,
  terminal, IME, accesibilidad ni la selección final del toolkit.
- Checks Rust Windows: formato, `cargo check` de producto y CLI, Clippy,
  build offline de Studio, `cargo test --no-default-features --features cli`
  (3 passed) y validación del schema v1 pasaron. `python -m compileall -q src`
  también pasó.
- Suite Python Windows: `uv run pytest` pasó 475, omitió 37 y deseleccionó 10.
  La prueba de symlink se omite solo si Windows devuelve `WinError 1314` al
  crearlo; en los demás casos sigue comprobando que no se siga el enlace.
- Las corridas E2E dejaron 120 procesos de servidores OpenCode falsos en TEMP;
  se cerraron por coincidencia exacta con `opencode-fake.py`; no quedan activos.
- Se detuvo por instrucción del usuario la ejecución Fedora 44 del Studio
  legado Qt: alcanzó 219/386 tareas y fue terminada por `podman stop` (salida
  137). No llegó a CTest, CPack ni al verificador; no es un resultado de fallo
  del código. Luego se eliminó `podman-machine-default`, liberando la imagen y
  disco de Fedora; Podman CLI sigue instalado. R0.4 sigue abierto y diferido
  hasta que el usuario lo ejecute en Fedora.
- El build parcial de 443 MB continúa en una carpeta única bajo `$TEMP`; la
  revisión automática bloqueó su eliminación recursiva. El usuario debe quitar
  `chxchx-r04-fedora44-*` de TEMP si quiere liberar también ese espacio.
- Los cambios candidatos de CPack Fedora siguen sin validar. Al retomarlos,
  separar la evidencia del artefacto legado Qt de la validación del producto
  Rust.
- Basic Memory/Serena y `.ai/SKILLS.md` no están disponibles en este checkout;
  el checkpoint se conserva aquí y en `.ai/CURRENT_STATE.md`.

## Continuación activa: R2.4 (2026-11-13, Windows)

- R2.4 ya tiene base automatizada. `migration/rust/tests/product_workflows.rs`
  ejecuta la CLI en subprocesos con proyecto, `CHXCHX_TECH_HOME` y `PATH`
  temporales. Cubre `init --minimal` preview/apply/idempotencia, trust
  preview/apply/revocación con backup y `projects list` con registro vacío o
  malformado preservado. No ejecuta Basic Memory ni otros clientes externos.
- R2.4 también añade al workflow de tres plataformas la suite CLI y una
  validación Draft 2020-12 de la salida real `status --json` con
  `jsonschema==4.26.0`. Local Windows: formato, tests (3 passed) y el script de
  schema pasan. Aún faltan resultados de la ejecución remota macOS/Linux.
- Corregido un fallo real de init en Windows: `directory_resolves_inside`
  compara ahora rutas canónicas normalizadas, sin prefijo `\\?\`. Corregida
  también la paridad del validador: `workspace.header.logo` permite cadena
  vacía como el modelo Python y como la plantilla Rust por defecto.
- Roadmap/matriz actualizados. El gate R2.4 queda abierto hasta una ejecución
  remota verde en los tres runners; R3.4 sigue abierto para preview obsoleto,
  casos completos, fallos parciales, rollback, preservación de contenido ajeno
  y mapeo de capacidades a presentaciones.
  R2.1 sigue parcial (`app/`, `runtime/`, `protocol/` faltan), R0.4 espera la
  corrección de CPack/Qt en Fedora y R1 requiere el prototipo editor/terminal.
- Siguiente: recoger los resultados remotos de la nueva matriz cuando CI ejecute
  estos cambios. Después retomar R0.4 con una compilación/paquete CPack real en
  Fedora 44; este checkout Windows no puede certificar ese gate. R1 aún requiere
  el prototipo interactivo de editor/terminal antes de ampliar el port.
- Se conservan como referencia los checkpoints y limitaciones de Python de la
  revisión anterior. Basic Memory/Serena no están disponibles.

## Punto activo para retomar (2026-11-13, Windows)

Esta sección define el siguiente trabajo. Las secciones inferiores se conservan
como checkpoints históricos: referencias a matriz ausente, init pendiente,
cambios sin commit en `dev` o `switch` como siguiente tarea ya no describen el
punto activo.

- Se revisaron AGENTS, PROJECT, CURRENT_STATE, ROADMAP, RUST-MIGRATION,
  RUST-CAPABILITY-MATRIX, PERFORMANCE-BASELINE, EDITOR-PARITY, RELEASE-ROADMAP,
  ADR 0016, README y el paquete/CI Rust. Rama `migracion-rust`, HEAD `619edfa`,
  árbol inicialmente limpio. No hubo cambios funcionales ni de configuración.
- La migración completa no está lista para cerrar ni para reemplazar el legado:
  solo hay un conjunto parcial de operaciones Rust compartidas; faltan editor,
  PTY/ConPTY, supervisor/procesos/recursos, agentes, skills/packs/mods, contexto,
  Git, CI/CD, preferencias, distribución y QA multiplataforma.
- R0.4 es el primer gate abierto: el paquete Fedora medido carece de Qt/plugins.
  Reproducir allí con la versión de Qt documentada, corregir el deploy/CPack de
  `native/CMakeLists.txt`, pasar `scripts/verify_studio_package.py`, volver a
  medir tamaño y completar presupuestos/mediciones pendientes. Este Windows no
  tiene CMake/Ninja/CTest en PATH; no se reprodujo el fallo Linux ni un build Qt.
- Después validar R1: árbol/editor/terminal/chat/permisos, IME, teclado,
  accesibilidad y builds/arranques por SO, antes de cerrar el ADR de GUI/editor.
- En Windows se puede adelantar R2.4: pruebas compartidas para capacidades ya
  portadas y validación del contrato `schemas/project-overview-v1.schema.json`
  en `.github/workflows/rust-product.yml`. Actualmente no hay suite Rust y la
  CI solo comprueba formato, compilación y Clippy. No posponer esta cobertura
  hasta después de `switch`/instaladores como sugieren checkpoints anteriores.
- R2.1 está marcado completo, pero `app/`, `runtime/` y `protocol/` no existen
  como módulos del paquete actual; la estructura y el supervisor siguen por
  materializarse. No asumir que ese check acredita runtime o IPC implementado.
- Mantener un paquete/versión y operaciones comunes; avanzar luego con R3–R7.
  No promover Rust a raíz ni retirar Python/Qt hasta cerrar los gates de paridad,
  instalación/actualización/rollback y QA.
- Validación Windows: `python -m compileall -q src` pasó con Python 3.13.0.
  `uv run pytest` (Python 3.12.14) terminó en 225.44 s: 475 passed, 36 skipped,
  10 deselected y 1 failed. Falló
  `tests/workspace/test_chat_context_inline.py::test_a_symlinked_context_file_is_not_followed`
  por `WinError 1314` al crear el enlace del fixture, antes de ejecutar el código
  del producto. Resolver la portabilidad de esa prueba o verificarla en un
  runner con permiso de symlinks; no contar esta ejecución como suite verde.
- Rust (Cargo 1.99.1): `cargo fmt --manifest-path migration/rust/Cargo.toml
  --all -- --check`, `cargo check --manifest-path migration/rust/Cargo.toml
  --locked --offline`, el mismo check con `--no-default-features --features cli`
  y `cargo clippy --manifest-path migration/rust/Cargo.toml --locked --offline
  -- -D warnings` pasaron. No se ejecutaron pruebas Rust, agentes reales, MCP,
  QA visual, CI remoto ni cambios sobre proyectos registrados.
- Basic Memory/Serena no están disponibles; checkpoint local en estos archivos.

## R0.4 medido parcialmente; paquete Fedora incompleto (2026-11-12)

- Resultados y reproducción: `.ai/PERFORMANCE-BASELINE.md`. Se midió Release
  con fixtures de 21 y 5,001 archivos en Fedora 44. En el fixture grande, CLI
  `status` p95 372 ms; Studio startup proxy p95 330 ms; RSS idle máximo 67.1
  MiB; índice p95 172 ms y contenido p95 136 ms.
- Se añadieron `scripts/perf/create_fixtures.py`,
  `scripts/perf/measure_processes.py` y el target opt-in
  `CHXCHX_BUILD_BENCHMARKS` para medir `ProjectFileIndex`.
- Pendiente para cerrar R0.4: investigar y reparar deploy/CPack Qt del TGZ
  Linux; hoy mide 4.31 MB, pero el verificador detecta Qt Core/Widgets/
  Core5Compat y plataforma xcb ausentes. Volver a generar y validar tamaño;
  fijar límite de paquete cuando sea distribuible. El deploy genérico omite
  dependencias Qt en `/lib64`; forzarlas no bastó porque falla ajustando el
  RPATH de un plugin Fedora sin entrada RPATH. El intento custom se revirtió.
- Presupuestos de comparación preliminares constan en el reporte. R1 es el
  siguiente gate después de cerrar este bloqueo de paquete. No se ejecutaron
  tests en esta medición. Basic Memory/Serena no disponibles.

## Baseline Fedora completado; siguiente R0.4 (2026-11-12)

- Python compileall pasó; `uv run pytest` pasó 511, saltó 1 y excluyó 10 live.
  El entorno uv usa Python 3.11.18; el host es Fedora 44/Python 3.14.10.
- Qt legado: GCC 16.2.1/CMake 4.3/Ninja 1.13/Qt 6.11; Studio compiló y CTest
  pasó 11/11. Se usó `env -u LD_LIBRARY_PATH`; warnings solo en dependencias
  Scintilla/Qt vendorizadas. Build temporal `/tmp/chxchx-native-baseline.NzU66P`.
- R0.3 queda evidenciado para Fedora. No cubre live, instaladores/release,
  interacción visual ni macOS/Windows/Ubuntu. Rust `cargo check --locked` pasó.
- Siguiente: ejecutar R0.4 con repositorios pequeños/grandes fijados y medir
  startup, RSS idle/activo, CPU idle, latencia p50/p95 y tamaño de paquete; no
  cerrar gates hasta guardar comandos, hardware, muestras y condiciones.
  Después cerrar prototipo R1 antes de diseñar `projects switch`/supervisor.
- No se hicieron cambios de código; Basic Memory/Serena siguen sin herramienta.

## Siguiente paso de migración Rust (2026-11-12)

- `.ai/RUST-CAPABILITY-MATRIX.md` ya documenta capacidades, contratos,
  autoridad, rutas globales y por proyecto, catálogos y QSettings. R0.2 queda
  documentado; R0.3/R0.4 y R1 todavía no pasan sus gates.
- Fedora 44 quedó comprobada con `rustc/cargo 1.100.0`; `cargo check
  --manifest-path migration/rust/Cargo.toml --locked` pasó. No se ejecutaron
  tests ni Clippy.
- Continuar con baseline reproducible del producto legado en Fedora: registrar
  versión/plataforma, checks disponibles y limitaciones; después medir startup,
  RAM/CPU/búsqueda/paquete en proyectos de tamaño fijo. No usar resultados
  históricos como evidencia del equipo actual.
- Luego completar el prototipo R1 de editor/terminal/IME/accesibilidad y decidir
  toolkit; `projects switch` queda detrás del diseño del supervisor/runtime.
- No se ejecutaron suites ni builds en este tramo. Basic Memory/Serena no están
  disponibles en la sesión.

## Revisión de brechas Rust (2026-11-12)

- Hallazgos: falta `.ai/RUST-CAPABILITY-MATRIX.md` aunque ROADMAP y
  RUST-MIGRATION la enlazan; R0.2–R0.4 y R1 siguen abiertos; CI no valida schemas
  ni ejecuta tests Rust. No dar por cerradas las fases de paridad.
- Próximo orden recomendado: restaurar/completar inventario y reproducir baseline
  (R0); cerrar prototipo GUI/editor/terminal y decisión ADR (R1); añadir pruebas
  compartidas y validación de contratos a CI (R2.4/R3.4); después acordar el
  supervisor y migrar `projects switch` con runtime de workspace/agentes.
- Luego siguen skills/packs/mods, adapters de agentes/contexto, Git, CI/CD,
  preferencias, instaladores y QA real en Windows/macOS/Ubuntu/Fedora. Sin cambios
  de código en esta revisión; Basic Memory/Serena no están disponibles.

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Estado y siguiente capacidad Rust

- Rama activa: `migracion-rust`; paquete unificado en `migration/rust/`.
  `project-overview`, detección/perfiles y preparación completa/compacta usan
  casos de uso compartidos por CLI/TUI/Studio.
- `init [PATH] [--minimal]` es preview por defecto y aplica solo con `--apply`.
  Completo genera plantillas faltantes, actualiza solo bloques administrados,
  migra TOML v1→v2, registra el proyecto y crea backups. Registro/cambios de
  archivos rechazan previews obsoletos y recuperan el estado ante fallos.
- `rollback [PATH]` también previsualiza por defecto; `--apply` restaura el
  último backup de archivos administrados después de guardar el estado actual.
  TUI: `b` previsualiza y `x` restaura. Studio expone las mismas operaciones.
- `workspace trust/untrust [PATH]` previsualiza y requiere `--apply`; vincula la
  aprobación al SHA-256 de la config, respalda el JSON global y detecta cambios
  entre preview y escritura. Sin config también se puede confiar explícitamente.
  TUI: `t`/`u`, luego `y`; Studio usa botones. Init no concede confianza.
- `integrate [PATH] --client all|claude|codex|opencode` comparte preview/aplicación
  MCP en CLI/TUI/Studio. Solo `--apply` invoca clientes oficiales y registra el
  proyecto Basic Memory. `--refresh` es visible en preview. Init completo crea
  el ejemplo de OpenCode como archivo administrado, con backup/rollback.
- Init completo y compacto también registran `.ai/memory` en Basic Memory al
  aplicar. La vista previa no ejecuta su CLI; si el registro falla, conserva
  los cambios locales y comunica un aviso en CLI/TUI/Studio. TUI y Studio
  ejecutan el apply en segundo plano y evitan mutaciones simultáneas.
- `doctor [PATH]` comparte reporte de solo lectura en CLI; TUI usa `d` y Studio
  ofrece un botón. Verifica herramientas, archivos, configuración/trust, cwd y
  MCP de Claude/Codex sin lanzar procesos configurados.
- `projects list/current` lee el catálogo local compartido desde CLI, TUI (`p`) y
  Studio. Lista alias, nombre, perfil, stacks, ruta, disponibilidad y último
  proyecto. El registro ausente es una lista vacía; JSON mal formado se reporta
  sin tocarlo.
- Validación Windows: compilación `cargo check --locked` default y CLI/TUI/Studio,
  Clippy, formato y `git diff --check` pasan. No se ejecutaron pruebas ni
  comandos externos en este tramo.
- La fase de paridad/pruebas R3.4 aún no empieza; la suite no se ejecutó. Queda
  anotada después de terminar `projects switch`, instaladores y las capacidades
  planificadas para R3.
- `projects sync` ya comparte preview y apply. Detecta snapshots obsoletos del
  registro/archivos, hace backup por proyecto y revierte si falla una escritura;
  TUI (`s`/`y`) y Studio ejecutan en segundo plano.
- Siguiente tramo: `projects switch` junto con runtime workspace/agentes, que
  debe suspender y arrancar los procesos con las políticas de trust/recursos.
  Después siguen instaladores.
  Mantener el producto legado como autoridad hasta cerrar paridad por capacidad.
- Local Windows: `cargo fmt --all -- --check`, `cargo check --locked` default y
  CLI/TUI/Studio por separado, Clippy con warnings denegados y `git diff --check`
  pasan. No se ejecutó la suite, Basic Memory, CLIs MCP ni CI; falta validar el
  comportamiento real con instalaciones y revisar macOS/Linux.
- Basic Memory/Serena no exponen `write_memory` en esta sesión; este checkpoint
  está guardado solo en los archivos `.ai/` versionables.

## Migración en curso: producto compartido Rust (2026-11-12)

- Rama `migracion-rust` activa; trabajar desde ahí. El producto Rust está en
  `migration/rust/` para evitar colisión con `src/` Python durante el port.
- Paquete Cargo único `chxchx-tech` (`0.6.0-dev.0`, interno/no publicado), con
  features y bins en un mismo manifiesto. No agregar crate ni versionado por UI.
- CLI/TUI/Studio comparten `features/project_overview::inspect_project`, incluida
  detección y compatibilidad inicial de configuración. El Studio es un prototipo
  visual nativo con overview + refresh; el TUI muestra la misma salida. Sigue
  pendiente probar interacción de ventana y teclado. Solo Windows fue compilado
  localmente.
- Contrato `chxchx.project-overview` v1 no es el bridge legado de status completo.
  Expandir por etapas y conservar ambos contratos mientras migra Qt.
- `.ai/RUST-CAPABILITY-MATRIX.md` contiene matriz/datos iniciales. Roadmap R0/R1
  no se cierra: faltan revisar claves QSettings/rutas, validaciones del legado,
  baseline local y prototipo de editor/terminal/accessibility.
- Check local: formato, `cargo check --locked` default/CLI/TUI y Clippy pasan en
  Windows. CI incluye formato/build/Clippy para Ubuntu/Windows/macOS, todavía sin
  resultados multiplataforma. No se agregaron ni ejecutaron tests en este turno.
- Siguiente capacidad: `init --dry-run` y preparación con backup/rollback; luego
  registro/recientes. No retirar Python/Qt hasta probar paridad por capacidades.

## Organización unificada y versiones (2026-11-12)

- Requisito aclarado: un producto Rust por capacidades, CLI/TUI/Studio como
  presentaciones internas; sin crates, roadmaps ni versiones independientes por
  interfaz. Se corrigieron RUST-MIGRATION y ADR 0016; Rust empieza bajo
  `migration/rust/` y solo se promueve a raíz tras el corte validado.
- Plan activo: `.ai/ROADMAP.md`, R0–R7. Empezar con PR de inventario R0.1–R0.2,
  después baseline R0.3–R0.4 y prototipo R1. No comenzar un port completo de una
  UI aislada. Cada capacidad debe incluir presentaciones aplicables y tests comunes.
- Versiones: `.ai/RELEASE-ROADMAP.md`; propuesta 0.6.0 alpha/beta/RC/estable para
  migración, 0.7–0.9 evolución y 1.0 estabilidad. Numeración y alcance sujetos a
  evidencia y a contrastar releases antes de publicar; versión actual intacta.
- Política propuesta: release común, builds con features, canales stable/dev,
  update compartido con preview, compatibilidad IPC/datos y rollback de binarios
  y datos. El roadmap anterior se preserva como referencia en history.
- Solo documentación/allowlist Git. Compileall, diff y enlaces locales correctos;
  `python -m pytest` no pudo ejecutar por módulo pytest ausente. Basic Memory y
  Serena no disponibles; no se alteraron el código ni la instalación Rust.

## Planeación de migración a Rust (2026-11-12)

- Entregados `.ai/RUST-MIGRATION.md` y ADR 0016 (propuesto). Requisito confirmado:
  CLI/TUI/Studio en Rust, Studio nativo sin WebView. No se comenzó el port.
- Próximo lote: inventariar capacidades/contratos/datos/dueños, medir baseline y
  construir un prototipo de editor + terminal antes de fijar la GUI. Iced es
  candidato, no decisión cerrada; evaluar Egui si no cumple requisitos.
- Mantener autoridad única por operación y paridad con Git Qt, modelos locales,
  CI/CD opt-in, permisos en vivo, trust, memoria, instaladores y rollback.
- Validación: `python -m compileall -q src`, enlaces Markdown y `git diff --check`
  correctos. Pytest no ejecutado: no está instalado en Python local; UV falló al
  descargar dependencias por restricción de red. El reintento con permisos
  ampliados se detuvo en preparación sin resultados. Quedaron `.venv` incompleto
  y `.cache/uv` locales, ignorados por Git; no hay cambios de código.
- No usar resultados históricos de Fedora como evidencia del entorno Windows
  actual. Basic Memory/Serena no disponibles; checkpoint guardado en estos archivos.

## Alta de agentes sin init y registro de fallos (2026-11-10)

- `agent add` ya no falla si falta `.ai/chxchx-tech.toml`: ejecuta `init --minimal` (solo toml + `.ai/memory`, con backup) y añade el agente; `--dry-run` solo previsualiza. Código en `workspace/agent_autoinit.py`. Tras añadirlo hay que volver a confiar el proyecto.
- Inicio muestra «Siguiente paso recomendado» (`native/src/main_window_guidance.cpp`): lista preparar → confiar → agente → skills → abrir, marca lo hecho según el estado real y un botón ejecuta el paso actual con los flujos de vista previa existentes. Compila y arranca offscreen; 11 smoke pasan; no se ha visto en ventana real.
- Studio registra fallos de arranque de agentes (`bridge report-error`, operación `studio:agent-start:<id>`), visibles en Más → Errores. Studio, tras confirmar `agent add`, abre solo la vista previa de trust (la config cambia el hash); el trust sigue siendo una confirmación explícita. Pendiente: probar en Studio real; los cambios siguen sin commit en `dev`. Suite: 510+1 passed tras el ajuste de test_cli.

## Abrir carpeta de proyecto (2026-11-09)

- Implementado sin commit en `dev`. Pendiente: probar a mano el diálogo y la ventana nueva (Fedora, macOS, Windows; en macOS el ejecutable dentro del .app debe relanzarse bien), añadir smoke Qt si se quiere cobertura y documentar el atajo en README/EDITOR-PARITY. `main_window.cpp` sigue sobre 300 líneas (excepción previa).
- Tip de entorno: con `LD_LIBRARY_PATH` apuntando a la instalación stable de Studio, el build falla con `Qt_6.11 not found`; compilar con `env -u LD_LIBRARY_PATH`.

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
