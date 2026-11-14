# Handoff

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Studio Rust: diseño verificado y Windows (2026-11-14)

- El diseño del Studio Rust (menú, barra de herramientas, explorador, héroe de Inicio) existe en `origin/migracion-rust`; la copia local de Fedora estaba 14 commits atrasada y además `chxchx-studio` del PATH es el lanzador del Studio Qt. Se actualizó y se verificó con captura: coincide con las capturas de Windows. Abrir siempre con `scripts/studio-rust.sh` (Linux/macOS) o `scripts/studio-rust.ps1` (Windows).
- Windows: `features/startup_log.rs` guarda errores de arranque y pánicos en `<home>/logs/studio-startup.log` (sin consola no se veían). CI nuevo `windows-package`: compila release, abre el Studio 8 s, lee `status --json` y sube el `.zip`.
- Pendiente: la prueba de integración falla en macOS desde `d48490b` (código 101; sin logs accesibles). Primera compilación debug local: ~43 min.
- El stash `local-before-pulling-origin-2026-11-14` guarda el rediseño anterior (superado) y notas `.ai` compactadas.

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
