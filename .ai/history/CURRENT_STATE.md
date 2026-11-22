# Historial de CURRENT_STATE

Secciones archivadas automáticamente, de la más reciente a la más antigua.

## Servicio de agentes y plan para retirar `old-version/` (2026-11-19)

- Decisión del mantenedor: Python/C++ no se erradican; donde aporten, viven como servicios bajo `services/<nombre>/` con contrato versionado y se conectan desde un adapter Rust. Lo que sobra se porta a Rust o se retira explícitamente, y `old-version/` se elimina.
- Nuevo servicio `services/agent-adapter/` (Python, solo biblioteca estándar): conduce Codex (`codex app-server`) y OpenCode (`opencode serve`, 1.x y 2.x) en modo en vivo. Contrato `chxchx.agent-adapter` v1 documentado en `contract.py`: una solicitud en la primera línea de stdin, respuestas y `cancel` en las siguientes, eventos `chxchx.agent-event` por stdout y un `adapter_done` final; el fin de stdin detiene el turno; solo ejecuta `codex app-server` / `opencode serve`. 50 pruebas propias (drivers con Codex/OpenCode falsos, contrato, CLI como proceso real) y job de CI en los tres sistemas.
- Adapter Rust: `integrations/agent_adapter.rs` (localiza el servicio —`CHXCHX_AGENT_ADAPTER`, junto al binario, `.venv` de un checkout, `PATH`—, exige la versión de contrato, detecta OpenCode 1/2 y construye la solicitud con la política de acceso de Rust, incluido el ruleset de OpenCode 2 en `agent_access.rs`) y `chat_bridge/service_runner.rs` (turno, handoff de solo lectura que rechaza tarjetas, cancelación, imágenes como adjuntos dentro del proyecto). Studio ya no necesita el CLI Python para Claude, Codex ni OpenCode si encuentra el servicio. Prueba Rust de extremo a extremo con el servicio real y un Codex falso (se omite sin `python3`).
- Verificado en Linux: formato, Clippy estricto, `cargo test --locked` (369 unitarias, 6 catálogo, 17 flujos), `cargo check --no-default-features --features cli` y `pytest` del servicio (50). Sin proveedor real de Codex/OpenCode, sin Windows/macOS ni CI remota de este cambio.
- Pendiente para borrar `old-version/`: (1) probar el servicio con Codex y OpenCode reales; (2) retirar la ruta heredada (`Turn::start`/`start_handoff`, `find_cli`, `supports_chat`, `legacy_cli`) y los comandos `agent chat*` de Python; (3) portar a Rust los comandos centrales aún delegados (`studio-update`, `projects switch`, `workspace attach/terminal`, `report-error`) y retirar explícitamente `ci`, `deploy`, `run` y el resto que ya no se use, documentándolo; (4) mover `install.sh`, `install.ps1` y `release_assets.py` fuera de `old-version/`, actualizar `release.yml` y `interfaces/installer.rs`, y empaquetar el servicio con el release (en Windows el MSI hoy trae un runtime Python para el CLI heredado); (5) borrar `old-version/` y actualizar README/MANUAL.

## Runner nativo de Claude y CI en verde (2026-11-19)

- La CI remota pasa en Ubuntu, Windows y macOS. Los fallos de las dos últimas rondas eran de los tests, no del producto: rutas temporales no canónicas (`RUNNER~1` en Windows, `/var` frente a `/private/var` en macOS), `/bin/true` inexistente en macOS, y `ps`/`sysctl`/`vm_stat` buscados en un `PATH` vacío (ahora se llaman por ruta absoluta del sistema).
- Studio ejecuta los turnos de Claude Code directamente desde Rust (`integrations/chat_bridge/claude_runner.rs`), sin el CLI Python: argumentos (`agent_access`), primer mensaje con contexto/recap/handoff (`chat_context`), imágenes copiadas a `.ai/chat-attachments/`, entrada stream-json en vivo, respuestas a permisos, cancelación del grupo de procesos y eventos `start`/`done`. El resumen de handoff de Claude también corre aquí, en solo lectura y sin entrada live. Trust: Studio lo verifica y el runner exige `TurnSpec.trusted`.
- Sin CLI Python Studio ya abre y conversa con Claude; Codex y OpenCode siguen necesitándolo. Verificado en Linux: formato, Clippy estricto, `cargo test --locked` (350 unitarias, 6 catálogo, 17 flujos), `cargo check --no-default-features --features cli` y una llamada real de una línea al `claude` instalado para confirmar el formato del stream. La CI remota de este cambio queda pendiente.
- Siguiente: runners Rust de Codex (JSONL no-live y app-server) y OpenCode (no-live y servidor), luego retirar el CLI Python delegado, el actualizador y los instaladores.

## Handoff automático de Studio en Rust (2026-11-19)

- Studio ahora calcula el umbral de liberación de contexto (90%), guarda y redacciona el handoff en `.ai/chat-handoffs/`, y lo pasa a una sesión nueva sin reanudar la anterior. Rust controla la validación, los límites, la compatibilidad de archivos previos y el consumo solo después de que la nueva sesión termina correctamente.
- Para redactar el resumen, Studio invoca un comando interno oculto del adapter Python en modo solo lectura, sin red, escritura de workspace ni aparición en el transcript visible. El CLI Python independiente todavía conserva su flujo legado; el adapter Python aún ejecuta los proveedores. Este slice no permite retirar `old-version/`.
- Verificación Linux: Rust `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` y `cargo test --locked` (338 unitarias, 6 catálogo, 17 flujos); Python `uv run --locked pytest -q` (561 pasaron, 1 omitida, 10 deseleccionadas). También pasan `git diff --check`. Sin proveedor real, QA nativa macOS/Windows ni CI remoto.
- La CI del primer push reveló que el build CLI sin GUI no habilitaba `time/parsing`, requerido por el parser RFC 3339 del historial OpenCode. `Cargo.toml` ya activa esa feature; el comando exacto `cargo check --locked --no-default-features --features cli` y el resto de la suite Rust pasan localmente. La segunda ejecución remota queda pendiente.
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

## Pendientes

- Confirmar el workflow de release en GitHub cuando haya acceso al CI remoto; la publicación aún no se ejecutó.
- No hay build Windows ARM64 y el panel Git sigue necesitando Git instalado. La distribución actual es de alcance personal, sin firma digital.

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

## Instaladores y canales de release (ADR 0013)

- `scripts/install.sh` (Linux/macOS) e `scripts/install.ps1` (Windows): canal stable/dev, `--version`, `--update`, `--cli-only`, `--uninstall`, `--dry-run`, verificación SHA-256 con el manifiesto, nada ajeno se sobrescribe. `scripts/release_assets.py` genera manifiesto + `SHA256SUMS`. Wrappers: `install-studio.*`, `install-studio-fedora.sh`, `bootstrap.*`. `windows-studio-setup.nsi` empaqueta `install.ps1` (`-DCHANNEL`).
- CMake lee la versión de `pyproject.toml`; Studio responde `--version`/`--help` antes de Qt; iconos en `native/packaging/` (png/ico/icns, `.rc` en Windows). Updater Python (`studio_channels.py`, `studio_update_launcher.py`) sigue el canal instalado; corregido que `parse_manifest` rechazaba `schema_version`.
- CI: `stage-release` → `installer-smoke` (ubuntu, macOS x2, Windows, servidor localhost + `CHXCHX_TEST_BASE`) → `publish-studio-preview` / `publish-studio-stable` (tag `v*`, con `check-release-tag`). Firma condicionada a secretos (`WINDOWS_CERT_*`, `MACOS_*`, `APPLE_*`).
- Verificado en Linux: `tests/installers` (37) y `tests/integrations/test_studio_channels.py` (8), build local de Studio con `--version`. **No verificado**: `install.ps1`, NSIS, flujo DMG, `installer-smoke` y publicación real, firma (sin pwsh/macOS/certificados aquí).

## Instalación local stable (2026-11-04)

- ChxChx Studio `0.5.0` instalado desde el release oficial stable en Fedora x86_64 usando `curl -fsSL https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.sh | bash -s -- --no-deps`. El primer intento sin `--no-deps` se detuvo porque `sudo` requería una terminal; las bibliotecas de Qt ya estaban instaladas. `--dry-run` y la instalación verificaron el manifiesto y los SHA-256.
- Studio informa `ChxChx Studio 0.5.0`; el bridge informa `chxchx-tech-lead 0.5.0`. Launcher y entrada de menú presentes; `~/.local/bin` ya está en `PATH`. El instalador configuró `uv` y Python 3.11 para el entorno aislado.
- La instalación anterior `dev-5eae006`, su launcher y su entrada de menú se retiraron tras confirmar sus marcadores de ChxChx. No se abrió la ventana gráfica; el arranque visual y la instalación en equipos limpios siguen sin comprobarse.

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
