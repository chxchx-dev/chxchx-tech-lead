# Current State

## Handoff automático de Studio en Rust (2026-11-19)

- Studio ahora calcula el umbral de liberación de contexto (90%), guarda y redacciona el handoff en `.ai/chat-handoffs/`, y lo pasa a una sesión nueva sin reanudar la anterior. Rust controla la validación, los límites, la compatibilidad de archivos previos y el consumo solo después de que la nueva sesión termina correctamente.
- Para redactar el resumen, Studio invoca un comando interno oculto del adapter Python en modo solo lectura, sin red, escritura de workspace ni aparición en el transcript visible. El CLI Python independiente todavía conserva su flujo legado; el adapter Python aún ejecuta los proveedores. Este slice no permite retirar `old-version/`.
- Verificación Linux: Rust `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` y `cargo test --locked` (338 unitarias, 6 catálogo, 17 flujos); Python `uv run --locked pytest -q` (561 pasaron, 1 omitida, 10 deseleccionadas). También pasan `git diff --check`. Sin proveedor real, QA nativa macOS/Windows ni CI remoto.
- Siguiente: portar runners y control live de Codex/OpenCode/Claude a Rust, y retirar paso a paso la dependencia Python de proveedores, instaladores/actualizador, CLI delegado y releases antes de eliminar `old-version/`.

## Retiro de `old-version` (2026-11-19)

- Se auditó el pedido de eliminar `old-version/`. En este checkout la migración aún no está completa: Studio todavía usa Python para los turnos de chat y actualizaciones; los modelos locales, su configuración y el catálogo de modelos/esfuerzos de Codex, Claude y OpenCode ya corren en Rust. El CLI Rust delega comandos pendientes y la release construye/publica el wheel Python. El MSI también incluye CPython, `uv` y el launcher auxiliar.
- Criterio actualizado por el usuario: el estado final no contiene archivos de producto en `old-version/`. Si Python o C++ son imprescindibles para una capacidad, se implementan fuera de esa carpeta como servicio independiente bajo `services/`, con contrato versionado, lifecycle, distribución y pruebas, y se conectan mediante un adapter Rust en `src/integrations/`. Rust conserva trust y control de procesos. No crear servicios auxiliares para evitar portar una capacidad que puede implementarse razonablemente en Rust.
- El inventario actual no muestra una dependencia obligatoria de Python/C++ en runtime: Codex, Claude, OpenCode, Ollama, Basic Memory y Serena son programas externos que Rust puede invocar por sus CLI/protocolos. No se crea un servicio auxiliar por ahora; se está portando el chat verticalmente antes de quitar el runtime Python.
- En el slice anterior, Rust Studio convirtió las decisiones visibles de permisos/preguntas Claude live al protocolo `control_response`, vinculadas al `request_id` original; las respuestas a subtipos no soportados se rechazan desde Rust. Rust es ahora dueño de los IDs pendientes y del cuerpo de respuesta; Python solo reenvía el JSON recibido. Codex app-server y OpenCode live siguen sin ese control Rust.
- Smoke de delegación en Windows: `target/debug/chxchx-tech.exe agent chat --help`, con `CHXCHX_TECH_CLI` apuntando a `old-version/.venv/Scripts/chxchx-tech.exe`, devuelve código 0 y anuncia el protocolo esperado. La prueba también reveló que Rich acortaba flags largos para stdout en tubería; el probe de Studio ahora fija `COLUMNS=200` y hay una prueba Unix para evitar la falsa incompatibilidad. Esta comprobación valida el descubrimiento/ayuda, no un turno con proveedor real.
- Smoke e2e de turno en Windows: `chxchx-tech.exe agent chat` atravesó Rust → CLI Python → un `claude.cmd` simulado; devolvió `start`, `session`, `message`, `usage`, `limits` y `done`, con código 0. Confirma argumentos, stdin, spawn y salida del provider adapter sin credenciales; no acredita todavía un turno real ni reemplaza el runner Python.
- No se consultó el actualizador contra la instalación del usuario: el `update_status()` Python llamado por `bridge studio-update` también ejecuta `_prune()` y puede borrar instalaciones antiguas. Separar consulta y limpieza o probar con una fixture aislada antes de dar esa invocación por validada.
- Verificación del slice anterior en Windows: la suite Python completa pasa con 500 aprobadas, 38 omitidas y 10 deseleccionadas. Las 21 pruebas enfocadas de Claude live y persistencia también pasan. Rust también pasó formato, Clippy, suite completa y compilación de tests Unix en ese seguimiento; el smoke `chxchx-tech agent chat --help` confirma el contrato Rust→CLI. No se ha ejecutado un turno con un proveedor real: el adapter Python todavía lanza Codex, Claude y OpenCode.
- El instalador visual Rust incorpora `old-version/scripts/install.sh`; README, MANUAL, Cargo, scripts de Windows, CI, manifiesto y `.gitignore` mantienen referencias al árbol.
- No se eliminó nada porque borrar la carpeta tal cual rompería esas rutas. Siguiente paso: portar o retirar explícitamente cada dependencia y después borrar el árbol completo. La carpeta incluye 387 archivos versionados y 71 MB en total, contando `.venv` y cachés ignorados.
- El usuario autorizó portar las capacidades faltantes, añadir y ejecutar pruebas unitarias y e2e y borrar la carpeta solo después de que pasen. El inventario incluye chat/protocolos de agentes, modelos locales, actualización, comandos de procesos/workspace y CI/CD, además del empaquetado; las superficies ya migradas se preservan.
- Procesos/workspace migrados parcialmente: `features/workspace_processes.rs` ofrece `process list/start/stop/log`; `workspace_lifecycle.rs` implementa `workspace start/stop/suspend/resume`, `auto_start`, trust, `--dry-run` y estados `ACTIVE/STOPPED/SUSPENDED/ERROR` en `workspace-state.json`. Los procesos con salida temprana se registran como `EXITED` o `FAILED`; se conserva `project_path`, `session_name` y `updated_at`.
- Recursos migrados parcialmente: `features/workspace_resources.rs` implementa umbrales RAM/swap, presupuesto de agentes, mediciones de procesos y el contrato read-only `chxchx.resources-overview` v1 mediante `resources [PATH] --json`; schema y CI actualizados. Linux mide RAM/swap y muestrea CPU entre lecturas; macOS mide RAM/swap y recursos por proceso, CPU de sistema queda `null`; Windows usa API nativa para RAM, commit aproximado como swap y RSS, CPU queda `null`. Windows ya pasó la suite local; faltan QA nativa macOS y confirmar CI remoto. El JSON conserva la ruta registrada o recibida mientras usa la ruta canónica para leer y deduplicar.
- Segundo flujo migrado parcialmente: `integrations/local_models.rs` diagnostica hardware y Ollama, estima encaje como el legado, y ejecuta setup/remove/configuración de OpenCode en Rust. CLI `local-models status/setup/remove`; Studio pide confirmación antes de descargar/quitar. Preview consulta el estado local, pero no descarga, borra ni escribe. RAM/disco incluye Windows; faltan QA nativa en Windows/macOS, más contratos/fallos y reemplazar el CLI Python para chat/actualizador.
- Chat y catálogo de agentes migrados parcialmente: `features/chat_state.rs` implementa `chxchx.chat-state` v1, con historial de texto acotado, filtro de roles, redacción de secretos, guardado atómico, reset y recap acotado. `features/chat_journal.rs` resume las conversaciones visibles, escribe `.ai/history/SESSIONS.md` con retención de 30 sesiones, deduplicación por sesión y límites de lectura/prompt; Rust agrega las dos entradas recientes al contexto de una sesión nueva. `features/explicit_memory.rs` detecta solicitudes explícitas en los transcripts Rust de Studio y las añade una sola vez a `.ai/memory/PROJECT_MEMORY.md`, preservando notas manuales y omitiendo bloques inyectados y posibles secretos. Studio registra memoria y diario al terminar cada turno; el adapter omite ambas escrituras duplicadas cuando recibe sus marcas Rust. El backfill histórico Codex/Claude/OpenCode se describe en el siguiente bullet. `integrations/agent_catalog.rs` lee en Rust los modelos y niveles de esfuerzo de Codex/Claude y consulta `opencode models`; Studio no invoca Python para poblar esas opciones. Studio guarda en Rust el mensaje visible antes del turno y las respuestas cuando termina; el adapter Python omite sus escrituras si Studio define `CHXCHX_RUST_CHAT_STATE=1`. `agent chat-state` y la carga/reset de Studio tampoco arrancan Python. `integrations/agent_stream.rs` normaliza en Rust registros JSONL de Codex y Claude, incluidos mensajes, herramientas, uso, errores, denegaciones y tarjetas Claude live; `agent_stream_opencode.rs` hace lo mismo para JSONL OpenCode no-live. El adapter puede enviar registros crudos con `CHXCHX_RUST_AGENT_STREAM=1`, pero mantiene su normalización interna para sesión/uso/contexto; Rust mantiene el estado de solicitudes live de Claude. Studio e2e recorre registros crudos Claude y OpenCode → Rust → transcript, memoria y diario. La ejecución de proveedores, control live de Codex/OpenCode, adjuntos y actualización aún dependen de Python. `agent_access.rs` y `agent_live.rs` tienen funciones puras y pruebas, pero todavía no están conectados al lanzamiento Rust de proveedores.
- Backfill de memorias privadas conectado para Codex, Claude y OpenCode: Studio lee solo mensajes del usuario, selecciona como máximo 10 conversaciones por proveedor para la raíz actual y aplica límites de archivos, mensajes, salida y timeout. OpenCode usa `session export` 2.x con fallback a `export` 1.x y `--sanitize`; abrir Studio y `/nuevo` hacen el trabajo en background. Las solicitudes explícitas se filtran, persisten con deduplicación y se añaden al contexto del chat.
- Contexto inicial del chat Studio conectado: `features/context_compaction.rs` compacta `CURRENT_STATE.md` y `HANDOFF.md` antes de una sesión nueva, conserva secciones canónicas/bloques administrados, archiva lo anterior en orden y crea backup antes de escribir. `integrations/chat_context.rs` arma el prompt, las notas acotadas y los resúmenes recientes del diario; antes de inlinear los archivos, `explicit_memory.rs` lee hasta 10 estados Rust del proyecto, recupera solicitudes explícitas de hasta 8 entradas y las persiste con deduplicación. El bridge entrega el contrato `chxchx.studio-chat-context` v1 por archivo temporal. Python desactiva la recuperación de memoria y del diario cuando Studio envía el contrato Rust y omite compactar otra vez; conserva esa ruta para su CLI independiente. `workspace memory-prune` limpia capturas automáticas con preview y backup. Las sesiones existentes no reconstruyen contexto. Si Rust no puede preparar el archivo, Python conserva la ruta de respaldo. El backfill de los tres proveedores ya corre en background; Studio también controla el ciclo del handoff automático mediante Rust.
- Reanudación de chat integrada parcialmente en Rust: `integrations/chat_resume.rs` verifica una vez la sesión restaurada, consulta `chat_history.rs` para Codex/Claude y `opencode session list` para OpenCode. Si la sesión desapareció, descarta su ID y construye un recap acotado desde el transcript visible. Studio entrega esa decisión al adapter con `--resume-decided-by-rust` y `--resume-recap-file`; Python no repite la consulta. `chat_history.rs` conserva límites de archivos y mensajes, raíz canónica y omisión de symlinks; `opencode_history.rs` lee OpenCode 1.x/2.x con export sanitizado, timeout y salida acotada. El handoff de Studio ya está en Rust; siguen pendientes el generador del resumen y el ciclo de handoff del CLI Python independiente, además de mostrar historiales de proveedor en la UI.
- El bridge limpia archivos temporales de prompt, recap y contexto incluso si falla el inicio del proceso o faltan tuberías; rechaza CLIs Python que no anuncian las opciones de reanudación y el contexto Rust v1. Smoke real en Windows: `chxchx-tech.exe agent chat --help` ejecutado desde el CLI Rust expone `--rust-context-file`.
- Chat history en Windows ahora busca carpetas Claude usando tanto la grafía original de la raíz como la canónica (incluidas rutas 8.3) y valida la propiedad del transcript contra la raíz canónica. La salida de recursos conserva la grafía de ruta que informó el registro/CLI.
- Se corrigió un cruce de índices en `old-version/scripts/install.sh`: al excluir binarios de Studio del catálogo de shims se desplazaba el destino de los comandos restantes y `chxchx-tech` podía ejecutar `chxchx-studio`. Los arrays de nombre y destino ahora se filtran juntos.
- Verificación Linux (2026-11-19): `cargo fmt --all -- --check`, Clippy estricto, `cargo test --locked` (338 unitarias, 6 de catálogo y 17 flujos de producto) y `git diff --check` pasan. Las pruebas nuevas cubren handoff legado con secretos, límites, cancelación y transferencia segura; no hubo proveedor real. macOS, Windows y CI remoto quedan pendientes.
- El script `scripts/validate_project_overview_schema.py` ahora valida ambos contratos; no pudo ejecutarse localmente porque falta el módulo Python `jsonschema`. CI instala su versión fijada. La prueba e2e de recursos sí valida el contrato y confirma que inspeccionar no escribe `workspace-state.json`.
- La migración sigue incompleta: el adapter Python aún ejecuta proveedores y genera el resumen de handoff; el CLI Python independiente conserva su ciclo legado. Parte del estado interno de permisos/preguntas y cancelación también sigue allí. Codex app-server y OpenCode live aún no tienen control Rust. También faltan CI/deploy, paridad de workspace/terminal, actualización y distribución sin runtime Python. No eliminar `old-version/`.
- No se pudo guardar memoria en Basic Memory: no hay herramienta `write_memory` disponible en este entorno.

## Instaladores visuales macOS/Linux y PATH de comandos (2026-11-17)

- Se añadió `chxchx-installer`, una ventana Iced que ejecuta la versión embebida de `install.sh`, permite elegir `stable`/`dev`, verificar descargas sin instalar, muestra la salida y ofrece instalar o desinstalar. CI prepara el DMG de `ChxChx Installer.app` para macOS arm64/Intel y el tar.gz Linux x86_64; `release_assets.py` los registra con SHA-256 en el manifiesto.
- `install.sh` agrega `~/.local/bin` de forma idempotente a Bash, Zsh o Fish. Linux usa `pkexec` desde la GUI solo para instalar bibliotecas del sistema. Los scripts de Windows y el builder MSI generan shims para futuros ejecutables CLI Rust `chxchx-*`; los paquetes macOS/Linux/Windows incorporan los binarios disponibles.
- Basic Memory y Serena siguen siendo extras requeridos, descargados durante la instalación; se conservan al desinstalar ChxChx. README y ADR-0018 describen la descarga de assets desde GitHub Releases. La distribución sigue sin certificado de firma digital ni notarización.
- Verificación local: `cargo fmt --all -- --check`, Clippy con `visual-installer`, `cargo test --locked` (218 + 6 + 9) y `uv run --locked pytest` (487 pasaron, 38 omitidas, 10 deseleccionadas) pasan. También pasan 18 pruebas enfocadas, sintaxis PowerShell y Bash del instalador (copia LF) y `git diff --check`. Windows omite la prueba end-to-end Linux de `install.sh`; se añadió su ejecución al workflow Linux de release. Pendientes: resultado remoto y builds nativos publicados de macOS/Linux.

## Checkout actual 0.6.3 (2026-11-17)

- Rama `dev`, versión Cargo y Python `0.6.3`. El MSI se construyó desde `dc80248` y su snapshot de distribución quedó en `9c2255c`; la release `v0.6.3` ya publica el MSI.

## Distribución Windows: MSI por usuario implementado localmente (2026-11-17)

- Se enlaza estáticamente el runtime MSVC para `x86_64-pc-windows-msvc`. Los tres ejecutables Rust y el launcher Python incorporan versión, manifiesto de ejecución sin elevación, DPI por monitor y soporte de rutas largas; Studio conserva su icono.
- El MSI WiX 5.0.2 instala por usuario en `%LOCALAPPDATA%\Programs\ChxChxStudio`, agrega PATH y acceso al menú Inicio, registra la instalación en Aplicaciones instaladas y trae CPython 3.11.18, `uv.exe`, el ayudante y sus paquetes. Su acción posterior a `InstallFiles` instala Basic Memory y Serena con `uv` en una carpeta compartida; sus comandos y PATH sobreviven a actualizaciones y desinstalación. `install.ps1` y `install.sh` también instalan ambos extras en la carpeta de herramientas del usuario; requieren Internet y se conservan al quitar ChxChx.
- El manifiesto admite `windows-msi`; las instalaciones MSI verifican el SHA-256 y actualizan mediante `msiexec`. El actualizador ZIP y `install.ps1` no modifican una raíz MSI.
- Verificado localmente en Windows: WiX 5.0.2 compila y valida con ICE el MSI 0.6.3 y el MSI de prueba 0.6.4; la prueba de ciclo con extras ahora comprueba sus ejecutables antes y después de actualizar/desinstalar. La ejecución completa de ese instalador de prueba se reserva al runner de CI porque instala los extras y deja su PATH persistente en el perfil que lo ejecuta.
- `cargo fmt --all -- --check`, Clippy y `cargo test --locked` pasan (218 unitarias, 6 de catálogo y 9 de flujos). La suite Python completa pasa en Windows: 487 pasaron, 38 omitidas y 10 deseleccionadas. También pasan la sintaxis PowerShell/Bash y `git diff --check`.
- La copia distribuible del MSI estable 0.6.3 está en `installers/windows/v0.6.3/`, versionada con Git LFS y acompañada por `SHA256SUMS` y el manifiesto. El MSI y los metadatos actualizados se publicaron en la release `v0.6.3`; sus digest SHA-256 coinciden con los archivos locales. La copia temporal de smoke test 0.6.4 permanece en `dist/windows-msi-smoke-update/` ignorada por Git. Por decisión del dueño, MSI y ejecutables se mantienen sin firma para uso personal y conocidos.

## Registro histórico de release estable 0.6.1 (2026-11-16)

- Se registró el tag `v0.6.1` sobre el merge de `main` `fa80b7c` (incluye `2c088e4`). El workflow `Studio release (Rust)` terminó correctamente y GitHub publicó paquetes Linux, Windows, macOS ARM64/Intel y wheel Python. Release: https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/v0.6.1.

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

- Producto: paquete Rust único con CLI, TUI y Studio Iced; `old-version/` conserva el ayudante Python, instaladores y actualización puenteados mientras se completa el port. La autoridad de las funciones nuevas está en Rust.
- Distribución Windows: se mantiene ZIP/`install.ps1` y se añadió el MSI per-user, aún pendiente de pasar por una release remota antes de estar disponible a usuarios.
- La suite de release publica los canales `stable` y `dev` con un manifiesto común. El CI remoto no se verificó en este checkout.
- Los límites de paridad del Studio y los portes pendientes permanecen en `.ai/EDITOR-PARITY.md` y `.ai/ROADMAP.md`.

## Pendientes

- Confirmar el workflow de release en GitHub cuando haya acceso al CI remoto; la publicación aún no se ejecutó.
- No hay build Windows ARM64 y el panel Git sigue necesitando Git instalado. La distribución actual es de alcance personal, sin firma digital.
