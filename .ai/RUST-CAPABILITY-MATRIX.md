# Matriz de capacidades para la migración a Rust

- Actualizada: 2026-11-12.
- Propósito: inventariar contratos, datos, autoridad y evidencia del producto
  legado frente al paquete único `migration/rust/`.
- Regla de estado: “Parcial” indica que existe un slice Rust; no certifica paridad
  ni permite cambiar la autoridad del producto.
- No se ejecutaron suites en esta revisión. Los enlaces a tests identifican la
  cobertura legado disponible, no resultados de ejecución actuales.

## Capacidad y autoridad

| Capacidad | Autoridad actual / evidencia legado | Slice Rust y contratos | Pendiente y puerta |
| --- | --- | --- | --- |
| Detección, perfiles y overview | Python: `core/detector.py`, `core/profiles.py`; tests `tests/test_detector.py`, `tests/test_profiles.py` | `features/project_detection.rs`, `project_profiles.rs`, `project_overview.rs`; `chxchx.project-overview` v1 | Paridad automatizada de detección/config; la schema no sustituye `chxchx.project-status` v1. |
| Configuración, init y preparación | Python: `core/project_config.py`, `config_migrations.py`, `bootstrap.py`, `preparation.py`, `templates.py`; tests `tests/workspace/test_config.py`, `tests/test_templates.py` | `project_config*`, `project_setup.rs`, `project_templates.rs`; preview/apply, migración v1→v2 y backups | Comparar todos los campos y efectos; probar idempotencia, fallos y rollback en copias aisladas. |
| Catálogo, cambio y sync de proyectos | Python: `core/registry.py`, `commands/projects.py`; tests `tests/test_registry.py`, `tests/test_sync.py` | `project_catalog.rs`, `project_registry.rs`, `project_sync.rs`; `projects list/current/sync` | `projects switch` requiere runtime compartido; validar datos malformados y registro obsoleto. |
| Trust | Python: `core/trust.py`; tests `tests/workspace/test_trust.py` | `project_trust.rs`; aprobación local ligada al hash de config, preview y backup del registro | Comparar semántica/hash y migración; nunca importar confianza con menor rigor. |
| Backups y rollback | Python: `core/backup.py`, `workspace/manager.py`; tests `tests/test_backup.py`, `tests/workspace/test_manager.py` | `project_backup.rs`, `project_rollback.rs`; rollback de archivos administrados | Cobertura común de fallos parciales y datos ajenos preservados. |
| MCP, memoria externa y diagnóstico | Python: `integrations/mcp.py`, `basic_memory.py`, `mcp_diagnostics.py`; tests `tests/test_integrations.py`, `tests/test_mcp_diagnostics.py` | `mcp_setup.rs`, `mcp_codex.rs`, `mcp_clients.rs`, `basic_memory.rs`, `doctor.rs`; configuración oficial Claude/Codex/OpenCode | Validar con CLIs instalados, sin credenciales de CI; completar contratos y pruebas falsas. |
| Procesos, workspace y recursos | Python: `workspace/process_manager.py`, `process_runtime.py`, `resources.py`, `service.py`; tests `tests/workspace/test_process_manager.py`, `test_process_runtime.py`, `test_resources.py` | Sin runtime Rust portado | Diseñar dueño por proyecto, límites/RAM Governor, exclusión, persistencia, crash/reconexión y cierre antes de `switch`. |
| Terminal y sesiones Zellij | Python: `adapters/terminal/`, `workspace/terminal_operations.py`, `zellij_tabs.py`; tests `tests/adapters/test_terminal.py`, `tests/workspace/test_layouts.py` | Sin PTY/ConPTY ni emulador VT Rust | Prototipo de R1 y luego transporte/emulación; preservar terminal externo como fallback. |
| Agentes, permisos y chat | Python: `adapters/agents/`, `integrations/agent_*`, `workspace/agent_chat.py`; tests `tests/e2e/test_*live*`, `tests/workspace/test_agent_chat.py`, `tests/integrations/test_agent_access.py` | Sin adapters de agente Rust | Codex, Claude, OpenCode/Ollama, eventos, aprobación/preguntas, cancelación, modelos, adjuntos y límites. Depende de R4. |
| Skills, Tech Packs y Mods | Python: `skills/`, `core/mods.py`, `core/mod_catalog/`; tests `tests/test_skill*`, `tests/test_tech_packs.py`, `tests/core/test_mods.py` | Sin catálogos portados; solo existe `assets/profiles.toml` | Empaquetar recursos Rust y portar detección, recomendaciones, apply/remove con backup (R3.3). |
| Memoria, historial y contexto | Python: `workspace/memory_history.py`, `session_journal.py`, `chat_context.py`, `chat_compaction.py`, `handoff.py`; tests `tests/workspace/test_memory_history.py`, `test_session_journal.py`, `test_handoff.py`, `test_chat_context_inline.py` | Sin capacidad equivalente completa | Portar lectura/escritura segura, historial, handoff, diario y reglas de retención sin transcripciones/secretos en logs. |
| Errores y logs | Python: `workspace/error_cache.py`, `process_manager.py`; tests `tests/workspace/test_error_cache.py` | Solo diagnósticos estáticos vía `doctor`; sin registro/runtime de errores | Definir retención, redacción, límites y consulta por proyecto durante R4/R5. |
| Git y asistente | Implementación propia de Studio: `native/src/git/`; ADR 0011/0012; tests `native/tests/git_smoke.cpp`, `git_ui_smoke.cpp` | Sin Git Rust | Portar status/diff, ramas, stash, remotos, asistente y backups; preservar argv seguro y locks por repo (R6). |
| CI/CD de proyectos | Python: `integrations/ci_*`, `deploy_*`; tests `tests/integrations/test_ci*`, `test_deploy*` | Sin capacidad Rust | Mantener opt-in, preview/confirmación, estado/logs y retiro; no confundir CD del producto con el del proyecto. |
| Modelos locales | Python: `integrations/local_model_*`, `ollama.py`; tests `tests/integrations/test_local_models.py`, `tests/e2e/test_local_models_flow.py` | Sin capacidad Rust | Portar inventario, ajuste, instalación/remoción y Ollama tras definir procesos/recursos. |
| Editor, búsqueda y preferencias | Studio legado: `native/src/` (Scintilla/Lexilla, búsqueda, símbolos, layouts); tests `native/tests/editor_smoke.cpp`, `content_search_smoke.cpp`, `symbol_index_smoke.cpp` | Studio Iced inicial; no editor, VT ni paridad de preferencias | Gate R1: prototipo, IME, accesibilidad, archivos grandes, teclado y costo de reemplazar Scintilla. |
| Distribución y actualización | `integrations/installers.py`, `studio_update_*`, CMake/CPack; tests `tests/installers/`, `tests/test_studio_package.py`, `native/tests/` | Sin instaladores/actualizador Rust | Un solo release por producto; instalar/actualizar/rollback/desinstalar en máquinas limpias (R7). |

## Datos, preferencias y rutas inspeccionados

### Datos globales del CLI/TUI legado

`core/paths.py` fija `$CHXCHX_TECH_HOME` (expandido y canonicalizado) como
override; el default es `~/.chxchx-tech-lead/`. Allí se encuentran:

| Ruta relativa | Uso / lector escritor principal |
| --- | --- |
| `projects.json` | Catálogo y proyecto reciente; `core/registry.py`. |
| `trusted_projects.json` | Trust por raíz canónica y SHA-256 de configuración; `core/trust.py`. |
| `workspace-state.json` | Estado persistido de workspace/procesos; `workspace/state.py`. |
| `backups/` | Backups por proyecto y operación; `core/backup.py`. |
| `logs/workspaces/` | Salida acotada de procesos administrados; `workspace/process_manager.py`. |
| `cache/errors.jsonl` | Errores locales recientes; `workspace/error_cache.py`. |
| `profiles/` | Perfiles locales que complementan los incluidos; `core/profiles.py`. |

La creación de home asegura `backups/`, `logs/` y `profiles/`; otras rutas se
crean al aplicar la operación correspondiente. El inventario no implica que el
formato o la ubicación deban cambiar durante el port.

### Datos locales por proyecto

- `.ai/chxchx-tech.toml`: configuración de workspace y comandos.
- `.ai/memory/PROJECT_MEMORY.md`, `.ai/history/SESSIONS.md` y otros documentos
  de contexto: memoria explícita e historial.
- `.ai/chat-handoffs/` y `.ai/chat-attachments/`: handoffs temporales y copias
  de adjuntos de chat.
- `.ai/cache/image-descriptions/`: descripciones de imágenes generadas por
  Studio; el producto legado las administra como caché regenerable.
- `AGENTS.md`, `CLAUDE.md`, `.claude/skills/` y `.codex/config.toml`: archivos
  administrados o integraciones por proyecto; conservar contenido ajeno y los
  bloques no administrados.
- `src/chxchx_tech_lead/skills/library/` y `core/mod_catalog/`: skills, packs y
  mods empaquetados con Python. En Rust solo está portado el catálogo de perfiles.

Las sesiones y configuraciones propias de Codex, Claude, OpenCode y Ollama son
datos de sus proveedores externos. La migración debe usar interfaces oficiales
y no asumir propiedad sobre sus bases de datos o secretos.

Rutas externas detectadas para lectura/diagnóstico, sujetas a variables de
entorno del proveedor: Codex usa `CODEX_HOME` o `~/.codex`; Claude usa
`CLAUDE_CONFIG_DIR` o `~/.claude`; OpenCode usa `XDG_CONFIG_HOME/opencode` y
`XDG_CACHE_HOME/opencode/models.json` (con defaults `~/.config` y `~/.cache`);
Ollama usa `~/.ollama/models`. No importar sus tokens, bases de datos ni cachés
como datos propios de ChxChx.

Rutas de instalación legado observadas en sus adapters/actualizador: Windows
`%LOCALAPPDATA%/Programs/ChxChxStudio`, macOS `~/Applications`, Linux
`~/.local/opt/chxchx-studio` y launcher en `~/.local/bin`. Son destinos de
instalación, no almacenamiento de proyecto; R7 debe conservar rollback/canales.

### QSettings del Studio Qt legado

La identidad declarada es organización `ChxChx`, aplicación `ChxChx Studio`
(`native/src/main.cpp`). Qt resuelve el archivo/registro por plataforma. Claves
encontradas mediante búsqueda de todas las llamadas `QSettings`:

| Alcance / clave | Contenido |
| --- | --- |
| Global `recentProjectFolders` | Hasta 10 rutas de proyecto recientes. |
| Global `editor/wordWrap`, `appearance/theme` | Preferencias de editor y tema. |
| Global `shortcuts/<action-id>` | Atajos editables; incluye `shortcuts/navigate.palette`. |
| Global `git/diffSideBySide`, `git/confirmPush` | Vista de diff y omisión recordada de confirmación de push. |
| Global `git/assistantAgent`, `git/assistantModel/<agent-id>`, `git/assistantConsent/<agent-id>` | Agente/modelo elegido y consentimientos del asistente Git. |
| Por proyecto `studio/projects/<sha256(raíz canónica)>/recentFiles` | Hasta 20 rutas recientes, validadas bajo la raíz. |
| Por proyecto `studio/projects/<sha256(raíz canónica)>/editor/{primary,secondary,activePath,splitOrientation,splitSizes}` | Sesión limpia de pestañas/editor; no restaura terminales ni agentes. |

Las claves identifican preferencias recuperables, no autorización para migrar
consentimientos como trust. Cualquier importación debe respetar el propósito de
cada valor y presentar preview si cambia la política o el ámbito.

## Gates abiertos y evidencia

- R0.1–R0.2 quedan documentados por esta matriz; la inspección exacta cubre
  registros, trust, archivos globales, datos por proyecto, catálogos y todas las
  llamadas `QSettings` encontradas en `native/src/`.
- R0.3 completado localmente el 2026-11-12 en Fedora 44 x86_64: `python -m
  compileall -q src tests scripts` pasó; `uv run pytest` dio 511 passed,
  1 skipped, 10 deselected (Python del entorno uv 3.11.18; host Python 3.14.10);
  CMake configuró y compiló el Studio legado con GCC 16.2/CMake 4.3/Qt 6.11;
  CTest pasó 11/11 smoke, incluido Git UI. `LD_LIBRARY_PATH` se retiró del
  entorno para evitar conflictos con Qt instalada; build temporal en `/tmp`.
  No usar resultados históricos como evidencia de este equipo.
- R0.4 tiene medidas parciales Release Fedora 44 en `.ai/PERFORMANCE-BASELINE.md`:
  fixtures de 21/5,001 archivos, CLI/Studio startup, RSS/CPU idle, indexado y
  búsquedas; se proponen límites preliminares para comparar el primer prototipo.
  El TGZ CPack (4.31 MB) falla el verificador por bibliotecas Qt/plugins xcb
  ausentes; falta corregirlo, medir el paquete válido y cerrar R0.4.
- R1 sigue abierto: evaluar árbol/editor/terminal/chat/permisos, IME,
  accesibilidad, archivos grandes, paquetes y builds/arranques por plataforma.
- R2.4 y R3.4 siguen abiertos: añadir pruebas comunes de capacidades, validar
  schemas/fixtures en CI y cubrir preview, idempotencia, plan obsoleto, rollback
  y errores. `migration/rust/` aún no tiene suite Rust de capacidades.
- `.github/workflows/rust-product.yml` compila y ejecuta Clippy en Ubuntu,
  Windows y macOS; también verifica CLI sin GUI. No incluye Fedora ni tests.
  El build Rust local en Fedora (`cargo check --manifest-path
  migration/rust/Cargo.toml --locked`) también pasó; aún faltan Rust tests y
  validar los otros runners en el estado actual.
- Quedaron fuera de R0.3: pruebas Python `live` (10 deselected y 1 skip),
  empaquetado de release, QA interactivo de la ventana y builds/pruebas macOS,
  Windows y Ubuntu en esta sesión.
- La columna de legado enumera ubicaciones de implementación/cobertura, pero no
  afirma que los tests actuales sean completos. R0.3 debe guardar comandos y
  resultados reproducibles; R3.4 debe mapear escenarios a las capacidades Rust.
