# Roadmap de ejecución: producto unificado en Rust

- Actualizado: 2026-11-12.
- Alcance confirmado: un producto Rust; CLI, TUI y Studio como modos de interacción.
- Studio: nativo, sin WebView. Toolkit pendiente de prototipo.
- Estado: trabajo incremental en `migracion-rust`; status, detección, init,
  rollback de archivos, trust local y setup MCP compartido están implementados
  parcialmente en la rama. La paridad global sigue abierta; el legado conserva
  la autoridad de producto.

## Documentos y orden de trabajo

- [RUST-MIGRATION.md](RUST-MIGRATION.md): arquitectura, estructura y transición.
- [ADR 0016](adr/0016-rust-control-plane.md): decisiones y propuestas pendientes.
- [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md): versiones y actualizaciones futuras.
- [EDITOR-PARITY.md](EDITOR-PARITY.md): baseline funcional a contrastar con código.
- [RUST-CAPABILITY-MATRIX.md](RUST-CAPABILITY-MATRIX.md): capacidades, evidencia y
  datos inspeccionados para el port.
- [Roadmap anterior](history/ROADMAP-PRE-RUST.md): referencia del legado, sin prioridad
  sobre este plan. Sus resultados históricos no acreditan validación actual.

La organización se hace por capacidades, no por tres aplicaciones. No se crean
crates independientes para CLI/TUI/Studio ni versiones por interfaz. Cada tarea
incluye su caso de uso, integraciones, pruebas y presentaciones aplicables.
La separación de presentación y dominio se mantiene dentro del paquete común.

## Secuencia de hitos

Los ítems se marcan completos con evidencia enlazada, no por existir un
prototipo o workflow.

### R0 — Inventario y baseline (fase 0)

- [x] R0.1 Inspección inicial de capacidades: proyecto/trust, procesos/recursos,
  agentes/permisos, conocimiento/contexto, Git, automatización y distribución.
  La matriz registra capacidad, implementación/contrato/datos, evidencia de pruebas,
  modos y dueños actual/destino; QA/dependencias se cierran en sus gates de fase.
- [x] R0.2 Cerrar inventario de configuración, registros, trust, QSettings,
  catálogos, caches y rutas. La matriz registra claves QSettings, ámbitos y
  formatos/rutas locales inspeccionados; quedan por validar datos en ejecución
  dentro del baseline R0.3/R0.4.
- [x] R0.3 Reproducir checks del legado y registrar limitaciones del entorno.
  Fedora 44: compileall correcto; `uv run pytest` 511 passed, 1 skipped,
  10 deselected; build Qt/C++ local correcto y CTest 11/11. Las pruebas live,
  empaquetado de release, QA interactivo y otros SO siguen fuera de esta
  evidencia local.
- [ ] R0.4 Medir arranque, RSS, CPU idle, búsquedas y paquete con repositorios
  pequeños/grandes y comandos reproducibles; fijar presupuestos de aceptación.
  Evidencia parcial Release Fedora 44: `.ai/PERFORMANCE-BASELINE.md` registra
  métricas, fixtures reproducibles y límites provisionales. Queda abierto porque
  el TGZ CPack no contiene Qt Core/Widgets/Core5Compat ni el plugin xcb y falla
  `verify_studio_package.py`; el deploy genérico filtra Qt en `/lib64` y falla
  al ajustar el RPATH del plugin Fedora al intentar incluirlo. Falta corregir y
  volver a medir el paquete.

Salida parcial: R0.1–R0.3 documentados/reproducidos; R0.4 tiene baseline
parcial. Falta un paquete Studio válido y tamaño aceptable antes de cerrar R0.
Dependencias: ninguna.

### R1 — Viabilidad del producto nativo (fase 1)

- [ ] R1.1 Prototipo con árbol, editor, terminal, chat simulado y permisos;
  ejecutar los escenarios del plan de migración, incluidos IME y accesibilidad.
- [ ] R1.2 Evaluar Iced; comparar Egui si hay límites. Documentar estrategia de
  editor/VT, licencias y costo de capacidades que hoy resuelve Scintilla.
- [ ] R1.3 Comprobar build/arranque Windows, macOS, Ubuntu y Fedora; separar
  evidencia real de plataformas aún no disponibles.
- [ ] R1.4 Cerrar ADR del toolkit/editor y alcance del primer build Rust.

Salida: viabilidad demostrada antes del port extenso; pendientes con costo y
resolución explícitos. Dependencias: escenarios y baseline de R0.

### R2 — Aplicación común y primera capacidad completa (fase 2)

- [x] R2.1 Crear un paquete Cargo en `migration/rust/`, con biblioteca común,
  `app/`, `features/`, `integrations/`, `runtime/`, `protocol/` e `interfaces/`.
- [x] R2.2 Definir primera API de lectura compartida, contrato y features
  `cli`, `tui`, `studio`; entrypoints finos y una sola versión.
- [x] R2.3 Entregar resumen de proyecto de solo lectura en los
  tres modos; misma acción y política, presentaciones adaptadas a cada modo.
- [ ] R2.4 Añadir CI de formato/lints/contracts y validación automatizada de
  capacidades; comprobar build CLI sin GUI y de los tres modos. Workflow de
  formato/build/Clippy agregado; falta validar el schema como contrato en CI y
  definir suite automatizada de capacidades.

Salida parcial R2.1–R2.3: el resumen usa el mismo DTO y lógica en los tres modos.
R2.4 y la paridad completa requieren terminar R0 y validar R1.

### R3 — Preparación y mutaciones seguras (fase 3)

- [ ] R3.1 Migrar configuración, init mínimo/completo y preparación. Detección
  de stacks/lenguajes/gestores/infraestructura y resolución de perfiles ya está
  en `migration/rust/src/features/`, compartida por los tres modos; falta
  demostrar paridad automatizada y migrar el resto del flujo. El resumen valida
  compatibilidad estructural inicial de TOML v1/v2. Init completo/compacto,
  plantillas, bloques administrados, migración v1→v2 y registro local ya tienen
  preview/aplicación compartidos y backup. `integrate` ya comparte preview y
  configuración MCP Claude/Codex/OpenCode; init completo genera el ejemplo MCP
  de OpenCode con backup. Init registra el directorio del proyecto en Basic
  Memory al aplicar; el preview no invoca el CLI y los fallos de registro quedan
  como avisos después de completar los cambios locales. `projects list/current`
  ya consulta el registro desde el mismo caso de uso en las tres interfaces;
  `projects sync` comparte preview/aplicación, snapshots, backup y rollback por
  proyecto. Faltan `switch` con runtime de workspace/agentes, instaladores,
  paridad automatizada y QA multiplataforma. `doctor` ya entrega reporte
  estático en CLI/TUI/Studio.
- [ ] R3.2 Migrar trust, preview/aplicación, backups y planes con versión de
  configuración; rechazar aprobaciones obsoletas. Init/rollback de archivos y
  registro ya verifican cambios desde preview y recuperan ante errores; trust
  explícito/revocación también verifica el hash de config y respalda el registro
  global. Faltan trust/preview para los otros dominios de mutación y paridad
  automatizada.
- [ ] R3.3 Migrar skills/packs/mods y empaquetar recursos fuera del paquete Python.
- [ ] R3.4 Cubrir los recorridos comunes en CLI, TUI y Studio usando las mismas
  operaciones; probar dry-run, idempotencia, fallos y rollback.

Salida: preparación segura equivalente al legado. Dependencias: R2.

### R4 — Workspace, procesos y terminal (fase 4)

- [ ] R4.1 Validar diseño de supervisor bajo demanda, IPC y dueño por proyecto.
- [ ] R4.2 Migrar recursos/RAM Governor, start/stop/resume y estado persistente.
- [ ] R4.3 Integrar PTY/ConPTY, emulación VT, resize, scrollback y launcher externo.
- [ ] R4.4 Probar dos modos abiertos a la vez, exclusión mutua, caída/reconexión,
  PID reutilizado, cierre de chat y cierre de interfaz sin procesos huérfanos.

Salida: ejecución compartida con política de ciclo de vida comprobada por SO.
Dependencias: R3 y prototipo terminal de R1.

### R5 — Agentes, conocimiento y contexto (fase 5)

- [ ] R5.1 Migrar adapters Codex/Claude/OpenCode/Ollama y eventos normalizados.
- [ ] R5.2 Conservar modelos/effort, aprobaciones, preguntas, cancelación, uso,
  adjuntos y caché de descripciones donde estén disponibles actualmente.
- [ ] R5.3 Migrar memoria, historial, contexto, diario, handoff y registro de errores.
- [ ] R5.4 Probar falsos E2E y smoke real controlado; presentar las acciones comunes
  desde todos los modos sin interpretar protocolos de proveedores en widgets.

Salida: sesiones y permisos equivalentes, datos preservados, sin exposición de
secretos en logs/fixtures. Dependencias: R4.

### R6 — Operación y experiencia integradas (fases 6–7)

- [ ] R6.1 Migrar Git, asistente, backups, remotos y serialización de escrituras.
- [ ] R6.2 Migrar modelos locales, CI/CD opt-in, diagnósticos y preferencias.
- [ ] R6.3 Completar editor, búsqueda, paletas, atajos, paneles y estados de
  carga/error; las acciones operativas siguen siendo capacidades comunes.
- [ ] R6.4 Cerrar matriz de paridad y QA interactivo en Windows/macOS/Ubuntu/Fedora,
  incluyendo operación por consola, teclado, accesibilidad y uso simultáneo.

Salida: capacidades existentes cubiertas; no un MVP con regresiones presentado
como reemplazo completo. Dependencias: R5; las tareas Git pueden adelantarse si
R3/R4 ya ofrecen sus garantías, sin cambiar el orden de las puertas de salida.

### R7 — Release, corte y retiro del legado (fase 8)

- [ ] R7.1 Aplicar prereleases y criterios de [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md).
- [ ] R7.2 Adaptar manifiesto/instaladores/actualizador, features distribuidas y
  versión común; preservar canales, marcadores y launchers existentes.
- [ ] R7.3 Probar instalar → abrir → actualizar → rollback → desinstalar en
  equipos limpios por plataforma; validar migración de datos y compatibilidad IPC.
- [ ] R7.4 Promover Rust a raíz, cambiar autoridad/launchers y retirar Python/Qt
  solo cuando sus operaciones estén cubiertas y la versión anterior sea recuperable.

Salida: producto Rust distribuible y recuperable sin Python/Qt propios.
Dependencias: R6 y validaciones de release. Publicar es una acción posterior,
no consecuencia automática de completar este documento.

## Primeras entregas pequeñas

1. **PR de inventario:** R0.1–R0.2; matriz y datos, sin mover código. El
   inventario está documentado; falta revisión y validación de runtime.
2. **PR de baseline:** R0.3–R0.4. Checks del legado y evidencia Fedora están
   registrados; faltan mediciones, fixtures de rendimiento y tamaño reproducible.
3. **PR de viabilidad:** R1; prototipo acotado y decisión de editor/GUI.
4. **PR de base común:** R2.1–R2.2; un paquete y sus fronteras.
5. **Primera capacidad en curso:** R2.3 entrega estado compartido; R2.4 tiene
   workflow de formato/build y necesita completar lints/contratos.

Cada PR declara capacidad, efectos, autoridad, presentaciones aplicables,
validación y rollback cuando corresponda. El implementador registra resultados
en CURRENT_STATE/HANDOFF. Sin fechas prometidas antes de medir el alcance.
