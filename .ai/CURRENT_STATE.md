# Current State

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
