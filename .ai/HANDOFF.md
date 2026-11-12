# Handoff

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

## Release 0.5.3 publicado (2026-11-08)

- Versión estable y lock sincronizados. Validación local: 511 passed, 1 skipped, 10 deselected; Studio y smoke nativo compilados, smoke nativo y `--version` correctos. El CI del tag pasó todas las matrices/builds y `installer-smoke` en Ubuntu, Windows y macOS; el release `v0.5.3` se publicó con 11 assets. Su primer intento reveló el ajuste de Rich en Ubuntu/Python 3.13, corregido con `soft_wrap=True`.
- Pendiente fuera de CI: probar el botón ✦ desde una ventana de Studio con un modelo de visión real.

## Imágenes pegadas y caché de descripciones (2026-11-08)

- Implementado **✦ Describir y guardar** a demanda con el agente/modelo actual en `--lean`. Genera Markdown con OCR, distribución, observaciones, inferencias e incertidumbres; lo guarda por SHA-256, caduca a los 30 días y limita a 200 entradas. Al readjuntar, el Markdown se usa por defecto; mientras el original esté disponible, el botón permite volver a enviarlo. La copia temporal del portapapeles y las copias de staging de consultas `--lean` se eliminan solo tras completar el guardado/turno; los archivos originales elegidos por el usuario se conservan.
- Smoke nativo cubre generación simulada, reutilización, vencimiento y eliminación solo de copias del portapapeles; la suite Python pasó (511 passed, 1 skipped, 10 deselected), `compileall` y el build completo de Studio también pasan. Pendiente: probar el botón con un agente/modelo de visión real en Studio y revisar la caché bajo `.ai/cache/image-descriptions/`.

## OpenCode y registro de errores (2026-11-08)

- Causa confirmada del mensaje mostrado: `agent add` necesita `.ai/chxchx-tech.toml`; sin ese archivo OpenCode todavía no está registrado en el workspace.
- Para habilitarlo en el proyecto afectado: `chxchx-tech init <PATH> --dry-run`, revisar y repetir sin `--dry-run`; luego `chxchx-tech agent add opencode --path <PATH>` o usar Añadir agente en Studio, y volver a confiar el proyecto.
- Errores de alta, preflight y chat quedan en la caché local. Consultar con `chxchx-tech workspace errors <PATH>` o `Más → Errores` en TUI/Studio. Suite completa: 511 passed, 1 skipped, 10 deselected.

> Historial anterior: `.ai/history/HANDOFF.md` (no se lee al iniciar un chat).

## Chat visual: aprobaciones en vivo (Claude, Codex y OpenCode hechos)

- Hecho con Claude (ADR 0014): permisos y `AskUserQuestion` en vivo. Probar a mano en Studio con el Claude real: aprobar, «en este chat», denegar, preguntas con varias opciones/selección múltiple, detener mientras espera, y cerrar la pestaña con una solicitud abierta.
- Resuelto: el contexto del chat nuevo se inyecta en el prompt (`inline_project_context`), así que ya no hay permiso para leerlo. Los archivos grandes se recortan (12 KB c/u, 36 KB en total).
- Hecho con Codex (`app-server`): probar a mano en Studio con el Codex real (comando fuera del sandbox, archivo fuera del proyecto, denegar, pregunta `request_user_input`), y vigilar cambios del protocolo experimental. El proceso es por turno; un proceso por chat permitiría «permitir en este chat».
- Hecho con OpenCode (`serve`): probar a mano en Studio con el OpenCode real (aprobar, «en este chat» con patrón, denegar —detiene el turno—, pregunta, cancelar con una solicitud abierta) y comprobar que no quedan procesos `opencode serve` tras cerrar la pestaña.
- Pendiente: carpetas adicionales (`--add-dir`). El reintento automático de la fase 1 ya conserva el borrador del cuadro de texto.

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
