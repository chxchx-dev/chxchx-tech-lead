# Handoff

## Release estable 0.6.1 (2026-11-16)

- El tag `v0.6.1` está publicado sobre `main` (`fa80b7c`) y la ejecución `43814711205` del workflow de release finalizó en éxito con los paquetes de todas las plataformas. Release: https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/v0.6.1.

## Studio: primeros pasos y scroll (2026-11-15)

- Versión `0.6.1` preparada en Cargo y el paquete Python auxiliar; changelog con la corrección de Inicio.
- Inicio tiene una guía compacta de preparación, confianza y primer agente; al completar una vista previa, el scroll principal va hasta el panel de revisión. Build de Studio y formato pasan.
- Pendiente de QA visual: comprobar rueda/trackpad en Inicio y otras páginas para medir la lentitud general con la misma máquina y contenido; aún no hay evidencia de que el uso de RAM sea la causa.

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

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
- Lo que aún es del CLI de Python (`agent`, `bridge`, `local-models`, `run`, `ci`…) se delega (`interfaces/cli_launch.rs`, `chat_bridge::legacy_cli`): `CHXCHX_TECH_CLI` primero, luego un `.venv`, y nunca el propio ejecutable; `CHXCHX_DELEGATED=1` corta cualquier bucle.
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
