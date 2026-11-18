# Handoff

## Eliminar `old-version/` (2026-11-18)

- El pedido es retirar el árbol completo y continuar con la estructura Rust. La inspección del checkout detectó dependencias todavía activas: chat y actualizaciones de Studio (`src/integrations/chat_bridge.rs`, `studio_update.rs`), delegación CLI (`interfaces/cli_launch.rs`; local-models ya está en Rust), instalador embebido (`interfaces/installer.rs`), wheel en `.github/workflows/release.yml` y runtime/launcher Python en `scripts/build_windows_msi.ps1`.
- Criterio aclarado por el usuario (2026-11-18): no debe quedar ningún archivo del producto bajo `old-version/`. Si Python o C++ resultan imprescindibles, se moverán a `services/<nombre>/` como servicio aislado, con protocolo versionado, lifecycle, empaquetado y pruebas propias; Rust lo descubrirá y controlará mediante un adapter en `src/integrations/`. No se aceptará que el servicio conserve dependencias de build o release en `old-version/`.
- El inventario actual no justifica un servicio Python/C++: los proveedores, Ollama, Basic Memory y Serena son programas externos con CLI/protocolos invocables desde Rust. Portar el adapter del chat a runners Rust; reconsiderar un servicio solo si al validar un proveedor aparece una dependencia técnica imprescindible.
- También hay referencias en `README.md`, `MANUAL.md`, `Cargo.toml`, `build.rs`, scripts de empaquetado y `installers/windows/v0.6.3/studio-manifest.txt`. `old-version/` contiene 386 archivos rastreados y ocupa 71 MB con artefactos locales ignorados.
- No borrar hasta resolver esas dependencias: eliminar solo el directorio dejaría Studio sin chat/modelos/actualizador y rompería los instaladores y releases. Portar las capacidades pendientes a Rust o confirmar cuáles se retiran; luego eliminar el launcher/wheel Python, actualizar instaladores, CI, documentación y manifiestos y retirar `.gitignore` obsoleto.
- El usuario confirmó que hay que portar todo lo pendiente, añadir y ejecutar pruebas unitarias y e2e, y eliminar `old-version/` únicamente después de que pasen. Plan: (1) inventariar y conservar contratos; (2) migrar servicios/CLI/Studio por flujos verticales; (3) quitar runtime/wheel/launcher Python de build, instaladores y release; (4) verificar Rust y e2e; (5) borrar árbol y repetir verificaciones.
- Procesos/workspace y recursos tienen slices nuevos: `process list/start/stop/log` y `workspace start/stop/suspend/resume` en Rust manejan `auto_start`, trust, preview y estados compartidos; los logs y recursos se leen con límites. `resources [PATH] --json` publica `chxchx.resources-overview` v1 con memoria y procesos registrados, sin escribir estado. Incluidos schema y validación CI.
- Modelos locales ya tienen slice Rust: diagnóstico de hardware/Ollama/OpenCode, evaluación equivalente del catálogo, CLI `status/setup/remove`, previews sin efectos, confirmación `--yes`, progreso JSONL, respaldo de `opencode.json` y confirmación visible en Studio. El e2e configura el modelo mediante Ollama simulado y comprueba `opencode.json`. `cargo check --locked --all-targets` y 13 pruebas enfocadas pasan.
- Historial de chat parcialmente portado: `features/chat_state.rs` conserva el contrato `chxchx.chat-state` v1, filtra roles, limita a 80 mensajes/8,000 caracteres, omite mensajes con marcadores de credenciales, escribe de forma atómica y construye recap acotado. Studio guarda en Rust el mensaje visible antes del turno y las respuestas al terminar; `CHXCHX_RUST_CHAT_STATE=1` indica al adapter que no escriba en paralelo. Studio y `agent chat-state` leen/resetan desde Rust. El proceso Python aún ejecuta los proveedores y conserva el protocolo del chat; falta portarlo antes de quitar `old-version/`.
- Base de contexto del turno en Rust: `integrations/chat_context.rs` arma la instrucción inicial, fija el ámbito Basic Memory, lee archivos con límites de bytes/caracteres sin seguir symlinks y prioriza handoff sobre recap. Sus 8 pruebas cubren esos contratos. No está conectada al runner: aún faltan en Rust compaction, recuperación/persistencia de memorias explícitas, diario de sesiones y handoff persistido; el adapter Python sigue siendo la ruta activa.
- Reanudación de chat integrada parcialmente: `integrations/chat_resume.rs` valida una vez las sesiones restauradas; usa `chat_history.rs` para Codex/Claude y `opencode session list` para OpenCode. Si el proveedor perdió la sesión y hay mensajes guardados, Rust descarta el ID y construye un recap acotado. Studio pasa la decisión al adapter mediante `--resume-decided-by-rust` y un archivo temporal `--resume-recap-file`; el adapter evita repetir su verificación. Las pruebas cubren sesiones válidas, perdidas, desconocidas, recap y el límite Rust→CLI→proveedor.
- Catálogo de modelos y esfuerzos portado a Rust: `integrations/agent_catalog.rs` consulta Codex (`config.toml` y `models_cache.json`), Claude (settings, aliases y transcripciones recientes) y OpenCode (`opencode models`). Studio ya no invoca `bridge agent-options`; la prueba e2e `a_conversation_from_opening_the_chat_to_the_answer` verifica que el recorrido tampoco llama ese comando. Esto no migra la ejecución de turnos, eventos ni permisos.
- Primer slice de eventos portado: `integrations/agent_stream.rs` normaliza JSONL de Codex y registros Claude a `chxchx.agent-event` v1 (sesión, mensajes, razonamiento, herramientas, uso, errores, denegaciones y permisos/preguntas live). Python conserva el cálculo interno para sesión/uso/contexto y el estado pending de Claude, pero Studio recibe `provider_record` y Rust construye los eventos visibles; Python todavía traduce las respuestas de Studio al protocolo del proveedor. Codex app-server y OpenCode live siguen usando sus normalizadores Python. Se cubrió el contrato con 5 unitarias Rust, e2e del bridge y Studio, y pytest que comprueba el traspaso y que los marcadores no se filtren al proveedor.
- OpenCode JSONL no-live también se normaliza en `integrations/agent_stream_opencode.rs`. El adapter delega esos registros cuando `CHXCHX_RUST_AGENT_STREAM=1`; la ruta conserva en Python el estado interno de sesión/uso y envía `provider_record` al host. Rust tiene 3 pruebas unitarias de contrato, una e2e de Studio con permisos denegados y uso, y el e2e Python verifica el transporte raw (25 pruebas enfocadas pasan). Los turnos live de OpenCode que Studio inicia siguen bajo el driver Python. `agent_access.rs` y `agent_live.rs` son funciones probadas, todavía sin integración al lanzador Rust.
- Studio también conserva ahora en Rust el mensaje visible antes de ejecutar el adapter y las respuestas al terminar. El proceso Python recibe `CHXCHX_RUST_CHAT_STATE=1` y omite las escrituras duplicadas; las pruebas de `tests/e2e/test_chat_persistence.py` mantienen su comportamiento normal cuando se ejecuta sin esa marca.
- La suite Python completa pasa: 552 aprobadas, 1 omitida y 10 deseleccionadas. Se ejecutó fuera del sandbox porque los e2e de Ollama levantan un servidor local.
- Próximo bloque de port: conectar `chat_context.rs` al turno y migrar memoria explícita, compaction, diario y handoff persistido. Después construir el lanzador Claude en Rust con sesiones, stream-json live, respuestas, imágenes, cancelación y contexto probado; luego portar Codex app-server JSON-RPC y OpenCode live v1/v2. `agent_access.rs` y `agent_live.rs` siguen como fundamentos probados sin lanzador Rust. La matriz y los tests de `old-version/` son la referencia de paridad.
- Evidencia enfocada del historial/catálogos: las pruebas unitarias de `chat_state` y `agent_catalog`, el e2e `chat_state_restore_and_reset_are_served_by_rust_without_a_python_cli`, `studio_restores_chat_state_in_rust_without_calling_python_chat_state` y `a_conversation_from_opening_the_chat_to_the_answer` pasan. `old-version/tests/e2e/test_chat_persistence.py` verifica que el adapter no escribe cuando Studio define el dueño Rust.
- Validación de este slice: formato, Clippy estricto y `cargo test --locked` pasan (292 unitarias, 6 de catálogo y 16 de producto). `uv run --locked pytest -q` pasa completo: 552 pasaron, 1 omitida y 10 deseleccionadas; las 23 pruebas enfocadas de chat incluyen el flujo e2e de recap desde el CLI. Studio e2e cubre Claude y OpenCode con registros crudos. Siguen pendientes los builds y pruebas nativos Windows/macOS, y confirmar CI remoto.
- Límites de recursos: CPU de sistema solo se mide por muestras Linux; macOS/Windows devuelven `null`. Windows calcula swap como aproximación desde el compromiso de memoria y requiere validación nativa.
- Pendiente para borrar `old-version/`: reemplazar ejecución y live drivers Python de Claude/Codex/OpenCode, portar memoria/contexto/historial restante, CI/deploy y actualización; cerrar attach/auto-open/editor, supervisor y suspensión de otros proyectos; paridad/plataformas de modelos locales; quitar wheel/runtime/launcher Python de instaladores y releases; retirar todas las referencias/manifiestos que leen el árbol. Solo borrar después de las pruebas unitarias, e2e y de distribución completas.
- La herramienta de Basic Memory `write_memory` no está disponible.

## Instaladores gráficos macOS/Linux y PATH por usuario (2026-11-17)

- El binario `chxchx-installer` (feature `visual-installer`) ejecuta el `install.sh` embebido, ofrece stable/dev, verificación `--dry-run`, instalación/desinstalación y salida en ventana. Release CI crea un DMG por arquitectura Mac y un tar.gz x86_64 para Linux; el manifiesto verifica ambos con SHA-256. El README tiene enlaces de descarga a `releases/latest/download/`.
- `install.sh` ahora agrega `~/.local/bin` a los archivos de inicio de Bash, Zsh o Fish con un bloque administrado idempotente y crea shims para todos los ejecutables CLI `chxchx-*` del paquete. MSI/ZIP Windows también generan los shims desde los binarios disponibles. Basic Memory y Serena se descargan como extras requeridos y se conservan al desinstalar.
- Linux GUI solicita `pkexec` solo para dependencias gráficas de sistema. Los paquetes principales siguen instalándose por usuario. Los instaladores no tienen certificado de firma; la app macOS solo lleva firma ad hoc y no está notarizada.
- Comprobado en Windows local: formato, Clippy con `visual-installer`, `cargo test --locked` (218 + 6 + 9), `uv run --locked pytest` (487 pasaron, 38 omitidas, 10 deseleccionadas), 18 pruebas enfocadas, parser PowerShell, `bash -n` sobre copia LF y `git diff --check`. La suite Windows omite el smoke end-to-end Linux de `install.sh`; ya quedó en el job Linux de release. Falta verificar el CI remoto, la apertura real de la GUI y publicar los assets nativos Mac/Linux.
- No se pudo guardar checkpoint en Basic Memory: el conector/herramienta `write_memory` no está disponible en este entorno; ADR-0018 y estos archivos locales guardan la decisión y el estado.

## Distribución Windows: MSI validado localmente (2026-11-17)

- Build MSI 0.6.3 desde `dev` (`dc80248`); el snapshot de distribución quedó en `9c2255c`. Además del runtime estático, recursos de versión/manifiesto, actualización MSI y protección entre instaladores, el MSI WiX 5.0.2 incluye CPython 3.11.18 y `uv.exe`; una acción por usuario instala Basic Memory y Serena en una carpeta compartida. `install.ps1` y `install.sh` invocan el mismo comando requerido y dejan los extras en el PATH del usuario.
- Build local completado para MSI 0.6.3 y actualización de prueba 0.6.4; ambos pasaron `wix msi validate` con ICE. El test `scripts/test_windows_msi.ps1` comprueba instalar, abrir los tres comandos, actualizar y desinstalar, conservando los extras y su PATH. No se ejecutó localmente porque modifica el perfil de usuario y deja intencionalmente los extras instalados.
- Verificaciones: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` (218 + 6 + 9); `uv run --locked pytest` (487 pasaron, 38 omitidas, 10 deseleccionadas); sintaxis PowerShell/Bash y `git diff --check`.
- Se añadió el MSI a la release `v0.6.3`, junto con el manifiesto y `SHA256SUMS` actualizados; GitHub confirma el nombre y el digest. La copia está versionada con Git LFS en `installers/windows/v0.6.3/`, con enlace desde README. El dueño mantiene MSI y ejecutables sin firma para uso personal y conocidos. El ciclo de instalación sigue pendiente de validación remota en Windows CI.

## Registro histórico del release estable 0.6.1 (2026-11-16)

- El tag `v0.6.1` está publicado sobre `main` (`fa80b7c`) y la ejecución `43814711205` del workflow de release finalizó en éxito con los paquetes de todas las plataformas. Release: https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/v0.6.1.

## Studio: primeros pasos y scroll (2026-11-15)

- Versión `0.6.1` preparada en Cargo y el paquete Python auxiliar; changelog con la corrección de Inicio.
- Inicio tiene una guía compacta de preparación, confianza y primer agente; al completar una vista previa, el scroll principal va hasta el panel de revisión. Build de Studio y formato pasan.
- Pendiente de QA visual: comprobar rueda/trackpad en Inicio y otras páginas para medir la lentitud general con la misma máquina y contenido; aún no hay evidencia de que el uso de RAM sea la causa.

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Conectores MCP (2026-11-16)

- Decisión del dueño: primera entrega con Excel, Power BI, bases de datos (PostgreSQL, SQLite), Word/Google Sheets y servidor propio; configuración **por proyecto**.
- Catálogo en datos: `catalog/connectors/*.toml` (embebido por `build.rs`, parseado en `features/mcp_connectors.rs`). Cada entrada dice comando (y forma `cmd /c` de Windows), requisitos (`npx`/`uvx`), variables de entorno (secretas o no) y parámetros (`{db_path}`). Los comandos salen de listados de terceros y del repo de cada proyecto: **Power BI (`--start`) y SQLite no están verificados contra el README oficial**; la vista previa muestra siempre el comando exacto. Faltan SQL Server y PowerPoint (sin paquete verificado); el «servidor propio» cubre cualquier otro.
- Escritura: `integrations/mcp_files.rs` (JSON de Claude `.mcp.json` y OpenCode `opencode.json`) y `mcp_codex::merge_servers` (TOML de Codex) son funciones de texto puras; `features/mcp_connectors_plan.rs` planifica/aplica con la transacción compartida (`apply_changes`, rollback si falla). Nunca se sobrescribe un servidor del mismo nombre configurado distinto, y al quitar solo se toca lo que coincide con lo que escribió ChxChx. Un secreto solo se **nombra** (`${VAR}` en Claude, `{env:VAR}` en OpenCode, `env_vars` en Codex).
- Respaldo propio en `<home>/backups-mcp/<proyecto>/<tiempo>/` (a propósito fuera de `backups/`: `rollback` toma el último de ahí y un respaldo sin AGENTS.md lo rompería). No lo restaura `rollback`; deshacer = `connectors remove`.
- Superficies: CLI `connectors` (`cli_connectors.rs`), TUI página Conectar (`tui_connectors.rs`, con prompt de parámetros), Studio sección en *Herramientas / MCP* (`studio_connectors.rs`). `serde_json` ahora con `preserve_order` para no reordenar los JSON del usuario.
- Pendiente: probar de verdad contra Claude Code/Codex/OpenCode (que arranquen los servidores), verificar comandos de Power BI/SQLite, SQL Server y PowerPoint, ADR corto, y mostrar los conectores en la ayuda del chat.

## 0.6.3: ajustes, temas y scroll de páginas (2026-11-16)

- Ajustes de Studio: `studio_settings.rs` (página), `studio_themes.rs` (6 paletas + acento; la paleta activa es global y los tokens de `design::` ahora son funciones), `studio_keymap.rs` (atajos remapeables; el listener es una función global). Se guardan en `studio-preferences.json` (`appearance`, `shortcuts`). Menú Archivo → Preferencias es un submenú (`Entry::Submenu`).
- Chat: historial ↑/↓, selección de texto por mensaje (editor de solo lectura), copiar con aviso, mensajes largos plegados, Markdown incremental.
- Catálogo de modelos (Python, `agent_catalog.py`): versiones de Claude y modelos de `opencode.json`; `opencode models` con 20 s de margen.
- Scroll: medido con XTEST bajo X11 en el Inicio, ~16,5 → ~8 ms/notch. Cambios en `vendor/iced_tiny_skia/src/engine.rs`: `fill_opaque_rounded` solo suaviza esquinas, `adjust_clip_mask` reescribe solo la zona sucia (hay prueba de equivalencia de píxeles). Degradados → color liso. Al subir iced hay que reaplicar los parches `ChxChx patch`.
- Pendiente: el terminal y el resaltado del editor no siguen el tema; las pistas de atajo de los menús son fijas; `claude-fable-5-1` y los ids 4.x del selector de Claude no están verificados.

## Scroll más ligero (2026-11-16)

- Medido con el Studio real bajo X11 inyectando rueda (XTEST) y muestreando pilas: el scroll del editor costaba ~13 ms de CPU por notch, casi todo en `tiny_skia::fill_path` con antialiasing y máscara. Causa: `iced_tiny_skia` dibuja **todo** cuadro de fondo como camino con AA; la tarjeta redondeada (radio 12, borde) de todo el grupo de pestañas (~1,4 M px) se rellenaba entera en cada fotograma.
- `vendor/iced_tiny_skia/` (copia de 0.14.1, MIT, `[patch.crates-io]` en `Cargo.toml`): rectángulos lisos van como `fill_rect`, y los rectángulos redondeados opacos se parten en un rectángulo central + dos tapas finas con las esquinas. Resultado: ~13 → ~8 ms/notch en el editor (−39 %), chat largo ~7 ms. Al subir iced hay que reaplicar el cambio (`ChxChx patch` en `src/engine.rs`) o descartarlo si ya lo incluye.
- `Event::Edit` ya no copia el documento entero para comparar en cada scroll/movimiento de cursor (solo en ediciones reales).
- Lo que queda por fotograma: dibujar ~4000 glifos (`draw_pixmap` crea un blitter por glifo) y presentar el buffer; bajarlo más exigiría cachear líneas de texto o reducir el área dañada.

## 0.6.0 y limpieza de old-version (2026-11-15)

- Versión `0.6.0` (Cargo y pyproject/wheel); `CHANGELOG.md` tiene la sección 0.6.0 (conserva el historial 0.5.x). Decisión del dueño: publicar 0.6.0 para que las instalaciones la bajen (reemplaza la alpha/dev planeada). Cada push a `dev` renueva `studio-dev-preview` (canal dev; solo lo reciben las instalaciones en dev). Pendiente de él: publicar el estable subiendo el tag `v0.6.0` (el workflow `release.yml` también se dispara con cualquier tag `v*` y publica el stable), o a mano en Actions con `channel=stable` (primero `publish` desmarcado para revisar). El stable publica el tag `v0.6.0` con las notas del changelog y `make_latest`. **El workflow nunca se ha ejecutado**; no se probó en macOS/Windows.
- `old-version/`: dejó de versionarse todo lo obsoleto (`native/` C++/Qt, `assets/`, `ci.yml.disabled`, scripts de su instalación, pruebas del TUI de Python y del paquete Qt), con entradas en `.gitignore`; sigue en disco de quien lo tenía. Se conserva lo que el producto usa: el CLI de Python, sus pruebas (515 pasan en un clon limpio), `install.sh`, `install.ps1` y `release_assets.py`. El wheel se construye bien desde ese árbol.

## Un solo comando `chxchx-tech` (2026-11-15)

- `chxchx-tech` sin argumentos abre Studio (escritorio, sin SSH) o el TUI (terminal) o la ayuda (tubería); `chxchx-tech <carpeta>` abre esa carpeta. Alias: `st dr sk ws pr t gui`, y `skill/pack/mod` en singular como el CLI de Python. `chxchx` es el mismo binario (`src/bin/chxchx.rs`).
- Lo que aún es del CLI de Python (`agent`, `bridge`, `run`, `ci`, `deploy`…) se delega (`interfaces/cli_launch.rs`, `chat_bridge::legacy_cli`): `CHXCHX_TECH_CLI` primero, luego un `.venv`, y nunca el propio ejecutable; `CHXCHX_DELEGATED=1` corta cualquier bucle. `local-models status/setup/remove` ya usa Rust.
- Instalación: el shim `chxchx-tech` (y el nuevo `chxchx`) apunta al binario Rust del paquete (`bin/chxchx-tech`, `Contents/MacOS/chxchx-tech`) y exporta `CHXCHX_TECH_CLI` con el Python como ayudante. Parcheados `install.sh`, `install.ps1` y `studio_update_launcher._rewrite_shim` (con tests); la prueba del bash se hizo con una instalación simulada, **`install.ps1` no se ha ejecutado** (sin PowerShell aquí) ni nada en macOS/Windows.

## CLI con color y diseño (2026-11-15)

- `interfaces/cli_style.rs` (`Ui`: color, glifos, banner) y `cli_view.rs` (lo que imprime cada comando; antes estaba en `cli.rs`). **Salida redirigida, `NO_COLOR` o `TERM=dumb` = el texto plano de siempre** (byte a byte); `CLICOLOR_FORCE`/`FORCE_COLOR` fuerzan el color. Truecolor si `COLORTERM`, si no los 16 colores; en Windows activa el modo VT o cae a plano.
- Con color: `status` como tarjeta con etiquetas, `doctor` por áreas con resumen, listas de skills/packs/mods alineadas, `version` con banner, errores en rojo, `--help` con colores y ejemplos. `--json` nunca se decora.
- Arreglo: `chxchx-tech … | head` ya no entra en pánico por tubería cerrada (SIGPIPE por defecto en `main.rs`).

## TUI rediseñado (2026-11-15)

- `interfaces/tui*.rs` (antes un solo archivo de 680 líneas): ocho páginas (Inicio, Preparar, Catálogo, Agentes, Git, Conectar, Salud, Proyectos) con pestañas, clic, ayuda (`?`) y la misma paleta que Studio. Cada cambio pasa por `tui_preview.rs` (vista previa + `y`), y aplicar corre en un hilo.
- Agentes: `Intro` entrega la terminal al agente y vuelve (`features/agent_launch.rs`, misma regla de confianza que Studio).
- Pruebas: pantalla en memoria (`tui_tests.rs`, `TestBackend`) con teclas y clics reales. Aplicar una vista previa no se prueba ahí (escribiría en el home real); lo cubre el e2e del CLI.
- `CHXCHX_TUI_PAGE=1..8` abre en esa página (demos y capturas). Pendiente: Salud no tiene filtro, Git es solo lectura, sin chat ni terminal embebidos (están en Studio).

## Alpha Rust en el canal dev y actualizaciones (2026-11-15)

- Versión `0.6.0-alpha.1` (Cargo) = `0.6.0a1` (pyproject/wheel). Decisión del dueño: alpha **solo en el canal `dev`**, según `.ai/RELEASE-ROADMAP.md`; ningún tag estable ni release publicado todavía.
- Publicar: Actions → *Studio release (Rust)* (`release.yml`, solo `workflow_dispatch`, canal `dev` o `stable`). Con `publish` desmarcado arma y deja los paquetes en el artefacto `release-assets` para revisarlos; marcado reemplaza el release rodante `studio-dev-preview`. **El workflow no se ha ejecutado nunca** (macOS/Windows sin probar); el staging y el manifiesto sí se probaron en local contra el actualizador instalado.
- Los paquetes siguen el formato del actualizador de las instalaciones 0.5.x (`bin/chxchx-studio`, `.app`, zip, wheel del CLI de Python), así que «Actualizar» de esas apps instala el Studio Rust. Solo lo reciben las máquinas en el canal `dev`.
- Actualizador (`old-version/.../studio_update_prune.py`): tras actualizar deja una sola instalación por canal (la nueva y la que estaba abierta). Arregla los `ChxChx Studio-0.5.2/0.5.3/0.5.4.app` acumulados en macOS; llega a cada equipo con el wheel de esta versión, no con las apps ya instaladas.
- Studio Rust: Ayuda → *Buscar actualizaciones…* (`studio_update.rs`, `integrations/studio_update.rs`) usa `bridge studio-update [--apply]`; en una compilación local explica que no es una instalación administrada.
- Pendiente: probar el workflow y una actualización real en macOS/Windows/Linux; firma y notarización de macOS; el `.app` va firmado ad-hoc.

## Studio Rust: chat visual, paneles, memoria y ruta de aprendizaje (2026-11-14)

- Chat visual (`interfaces/studio_chat*.rs`, `integrations/chat_bridge.rs`, `features/chat_model.rs`): usa `chxchx-tech agent chat --live` del CLI de Python; permisos en vivo, niveles de acceso, imágenes (`integrations/chat_images.rs`, se reducen a 1568 px), tokens/contexto/plan.
- Paneles acomodables (`studio_panes*.rs`, `pane_grid`): editor, chats, terminales y actividad se arrastran, dividen, redimensionan y maximizan. La disposición aún no se guarda entre sesiones.
- Memoria (`integrations/process_memory.rs`, `MEMORIA.md`): devolución al sistema, `MALLOC_ARENA_MAX`, recortes en el chat, RAM en la barra de estado.
- Ruta de aprendizaje en Inicio (`studio_learning.rs`).
- Sin probar con un agente real desde la ventana: permisos en vivo, preguntas, liberación de contexto.

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
