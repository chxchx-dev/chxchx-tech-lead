# Current State

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

## Inicio de migración Rust en `migracion-rust` (2026-11-12)

- La rama `migracion-rust` existe desde `main` y está activa. Conserva los
  cambios documentales pendientes previos de esta planeación.
- Agregado `migration/rust`: un paquete `chxchx-tech` con features CLI/TUI/Studio
  y una biblioteca compartida. Versión interna `0.6.0-dev.0`, no publicada.
- Primer slice `project-overview`: CLI (`status [PATH] [--json]`), Ratatui (`tui`)
  e Iced (`studio open`) consumen el mismo servicio/DTO; lectura de root/config,
  marcadores superiores, Git y SHA-256 de trust local. Schema dedicado
  `chxchx.project-overview` v1; no representa aún paridad con el bridge legado.
- Iced 0.14 software renderer necesita executor `thread-pool`. Los paquetes
  quedaron fijados por `Cargo.lock`. Workflows `.github/workflows/rust-product.yml`
  para formato y build (Ubuntu, Windows, macOS), sin ejecución de tests en este
  incremento. Se confirmó build local Windows; otros targets pendientes.
- Inventario inicial de capacidades/datos en `.ai/RUST-CAPABILITY-MATRIX.md`;
  R0.1 inicial completo y R0.2, R0.3/R0.4 pendientes. R1 prototipo visual aún
  no pasa editor/accesibilidad/QA. R2.1–R2.3 implementados; R2.4 parcial.
- Próximo trabajo: terminar inventario, completar build CLI sin GUI y lint;
  extender `projects` por la misma fachada para detección/configuración sin
  mutación. Trust existente solo se lee; no se alteró la confianza del usuario.
- No se crearon ni ejecutaron pruebas por el alcance de validación de este turno.
  Compile y formato verificados; suite existente aún no ejecutada.

## Producto Rust unificado y roadmap de versiones (2026-11-12)

- El usuario aclara que CLI/TUI/Studio deben pertenecer a un proyecto organizado,
  sin separarlos como productos. El plan y ADR 0016 ahora proponen un paquete
  Cargo con biblioteca común, capacidades en `features/` y presentación bajo
  `interfaces/`; reemplaza la propuesta inicial de crates por interfaz.
- `.ai/ROADMAP.md` es el plan activo con R0–R7, dependencias, checklists y primeras
  entregas pequeñas. El roadmap previo queda en `.ai/history/ROADMAP-PRE-RUST.md`.
  Nada de implementación se marca terminado. El siguiente paso es R0: inventario
  de capacidades/datos y baseline, seguido de viabilidad GUI/editor/terminal.
- `.ai/RELEASE-ROADMAP.md` propone 0.5.x legado, 0.6.0 alpha/beta/RC → Rust estable,
  0.7 fluidez/accesibilidad, 0.8 LSP, 0.9 revisión y 1.0 contratos/soporte. Son
  objetivos, sin fechas ni versiones publicadas; comprobar números antes de tags.
- Una versión, manifiesto y actualización por producto; features de build para
  consola/completo. Actualización con preview, sesiones activas, compatibilidad
  de datos, backup y rollback. No se modificó código, toolchain ni versión actual.
- Validación de esta ampliación: compileall y diff correctos; enlaces locales
  comprobados. `python -m pytest` no ejecuta tests porque falta pytest en Python
  local. No se repitieron descargas que fallaron en la entrega previa.
- Basic Memory/Serena siguen sin herramientas disponibles; checkpoint en archivos.

## Planeación de migración a Rust (2026-11-12)

- El usuario solicita reestructurar núcleo, CLI, TUI y Studio con Rust y confirma
  Studio nativo sin WebView. Solo se realizó planeación; no se cambió código ni
  se instaló/configuró el toolchain Rust.
- Plan versionable en `.ai/RUST-MIGRATION.md`, ADR 0016 propuesto y enlaces desde
  PROJECT/ROADMAP/EDITOR-PARITY. Stack candidato: Clap, Ratatui/Crossterm e Iced;
  GUI/editor/VT y runtime compartido requieren prototipo antes de su adopción.
- Próximo paso: matriz de paridad/autoridad y baseline, luego prototipo visual.
  Python/Qt siguen siendo la implementación actual. Git nativo también debe
  migrarse; no basta con portar el bridge Python.
- Basic Memory y Serena no están expuestos como herramientas en esta sesión;
  checkpoint conservado en archivos locales del proyecto.
- Validación actual: `python -m compileall -q src`, enlaces Markdown locales y
  `git diff --check` correctos. Pytest no se ejecutó: no está instalado en Python
  local y `uv run pytest` falló descargando dependencias por restricción de red;
  el reintento con permisos ampliados se detuvo durante la preparación, sin
  resultados. UV creó un `.venv` local incompleto y caché ignorados por Git.

## Alta de agentes sin init y registro de fallos (2026-11-10)

- `agent add` ya no falla si falta `.ai/chxchx-tech.toml`: ejecuta `init --minimal` (solo toml + `.ai/memory`, con backup) y añade el agente; `--dry-run` solo previsualiza. Código en `workspace/agent_autoinit.py`. Tras añadirlo hay que volver a confiar el proyecto.
- Inicio muestra «Siguiente paso recomendado» (`native/src/main_window_guidance.cpp`): lista preparar → confiar → agente → skills → abrir, marca lo hecho según el estado real y un botón ejecuta el paso actual con los flujos de vista previa existentes. Compila y arranca offscreen; 11 smoke pasan; no se ha visto en ventana real.
- Studio registra fallos de arranque de agentes (`bridge report-error`, operación `studio:agent-start:<id>`), visibles en Más → Errores. Studio, tras confirmar `agent add`, abre solo la vista previa de trust (la config cambia el hash); el trust sigue siendo una confirmación explícita. Pendiente: probar en Studio real; los cambios siguen sin commit en `dev`. Suite: 510+1 passed tras el ajuste de test_cli.

## Abrir carpeta de proyecto en Studio (2026-11-09)

- **Archivo → Abrir carpeta de proyecto…** (`Ctrl+Shift+O`) y **Archivo → Proyectos recientes** (hasta 10, en `QSettings`). Abren la carpeta en una ventana nueva de Studio (`QProcess::startDetached` del propio ejecutable), sin tocar terminales/agentes del proyecto actual; Inicio ofrece `Preparar y entrar` y el trust sigue explícito. Código en `native/src/main_window_projects.cpp`. Compila y arranca offscreen; no se ha probado con la ventana real ni en macOS/Windows.

## Release 0.5.3 publicado (2026-11-08)

- Versión estable fijada en `pyproject.toml` y `uv.lock`. Suite Python: 511 passed, 1 skipped, 10 deselected; compilación de Studio y `chxchx-agent-chat-smoke` pasan con Qt 6.11. El binario local informa `ChxChx Studio 0.5.3`; el tag coincide con la versión.
- Incluye el diagnóstico/registro local de errores de OpenCode y el pegado de imágenes con caché Markdown descritos abajo. El primer CI encontró un ajuste de línea de Rich en Ubuntu/Python 3.13; `soft_wrap=True` lo corrige. El run 42507440204 pasó las matrices, builds nativos y smoke de instaladores; release `v0.5.3` publicado con 11 assets. Falta QA visual con un modelo de visión real.

## Pegado de imágenes en Studio (2026-11-08)

- El compositor acepta imágenes `image/*` aunque Qt no las marque con `hasImage()`, y archivos de imagen copiados como URL local. **✦** genera una descripción con el agente actual en `--lean`, solo lectura y sin sesión; la guarda atómicamente por SHA-256 en `.ai/cache/image-descriptions/` (30 días, máximo 200). Al reutilizarla, manda el Markdown en lugar de la imagen; conserva la opción de enviar el original cuando existe. Elimina la copia del portapapeles solo después de persistir la descripción; no toca imágenes fuente del usuario. Los turnos `--lean` limpian sus copias de `.ai/chat-attachments` al terminar.
- Smoke Qt cubre generación, caché, reutilización, envío de imagen original, borrado de la copia pegada, vencimiento y MIME crudo; OpenCode 1.x/2.x cubiertos con E2E. Suite completa: 511 passed, 1 skipped, 10 deselected; `compileall`, build de Studio y smoke Qt pasan. Falta probarlo desde una ventana real de Studio con un modelo de visión.

## OpenCode y registro de errores (2026-11-08)

- `agent add opencode` identifica la ruta del TOML faltante y muestra la inicialización con `--dry-run`. ChxChx registra errores de alta, preflight y chat en la caché local; `chxchx-tech workspace errors <PATH>` permite consultarlos, igual que `Más → Errores` en TUI/Studio. La caché redacta credenciales habituales.
- Para usar OpenCode en un proyecto sin configuración, inicialízalo primero y vuelve a añadir OpenCode; después vuelve a confiar el proyecto porque cambia el hash de configuración.
- Suite completa verificada: 511 passed, 1 skipped, 10 deselected. `python -m compileall src` y `git diff --check` también pasan.

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Chat visual: seguridad y acceso (ADR 0014)

- `agent_access.py` (niveles → banderas por CLI), `agent_permissions.py` (evento `permission`), `agent chat --network/--allow`. Studio: selector de 4 niveles, botón 🌐 Red, `ChatPermissionCard`, confirmación de acceso total, concesiones «una vez» / «en este chat» (`AgentChatWidget::answerPermission`).
- Fase 2 (Claude en vivo): `agent_live.py` (`LiveClaude`: control_request ↔ control_response), `agent chat --live`, `ChatQuestionCard`, `answerPermission`/`answerQuestion` escriben JSON por stdin. Probado con un `claude` falso que espera al host (`tests/e2e/test_claude_live_flow.py`, 3 mutaciones) y con Claude real.
- Codex en vivo: `agent_codex_live.py` (`LiveCodex`, cliente JSON-RPC de `codex app-server`), `codex_server_settings` (nivel → sandbox/aprobación), `answer` con `scope`. Probado con un `codex app-server` falso (`tests/e2e/test_codex_live_flow.py`, 4 mutaciones) y con Codex real (aprobar fuera del sandbox, denegar).
- OpenCode en vivo: `agent_opencode_live.py` (`LiveOpenCode`: `opencode serve` + SSE + replies HTTP, con contraseña por turno), `agent_chat._json_lines/_live_driver`. Probado con un `opencode serve` falso (`tests/e2e/test_opencode_live_flow.py`, 5 mutaciones) y con OpenCode real (permiso permitido y denegado, pregunta; sin procesos huérfanos).
- Pruebas: `tests/integrations/test_agent_access.py`, e2e de OpenCode, `agent_chat_smoke` (3 mutaciones detectadas). Verificado con claude/opencode/codex reales salvo `bypassPermissions` y `danger-full-access`.

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
