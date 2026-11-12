# ADR 0016: Núcleo e interfaces en Rust

- Estado: Propuesta; dirección Rust nativo confirmada por el usuario, diseño
  técnico y selección de GUI pendientes de validación.
- Fecha: 2026-11-12.
- Plan: [RUST-MIGRATION.md](../RUST-MIGRATION.md).
- Ejecución: [ROADMAP.md](../ROADMAP.md); versiones:
  [RELEASE-ROADMAP.md](../RELEASE-ROADMAP.md).

## Contexto

El usuario solicita reestructurar completamente el proyecto con Rust para CLI,
TUI y Studio, y confirma una interfaz de escritorio nativa sin WebView. Hoy
Python mantiene la autoridad de las operaciones compartidas y Qt/C++ implementa
Studio, incluyendo capacidades propias de Git, edición y terminal.

El usuario aclara que no quiere CLI, TUI y Studio como proyectos separados:
la estructura debe pertenecer a un solo producto, organizada por capacidades.

La arquitectura vigente en AGENTS.md, PROJECT.md y EDITOR-PARITY.md sigue
describiendo la implementación actual. Este ADR propone reemplazar su destino
de núcleo C++ por Rust; no afirma que la migración ya exista.

## Propuesta

- Un paquete Cargo de producto con biblioteca interna común; capacidades bajo
  `features/`, composición en `app/`, integraciones y runtime compartidos.
- CLI/TUI/Studio son módulos de presentación bajo `interfaces/`, con dependencias
  opcionales por modo, sin crates ni versiones independientes. Los entrypoints
  y launchers son finos; todas las operaciones usan la misma fachada/catálogo.
- Migrar por capacidad con sus presentaciones aplicables. La separación entre
  dominio, casos de uso y adapters se conserva mediante módulos y verificaciones
  de arquitectura; no se sustituye por un archivo monolítico.
- CLI con Clap y TUI con Ratatui/Crossterm como candidatos; Iced como candidato
  principal de Studio y Egui como alternativa. Decisión de GUI tras prototipo
  de editor, terminal, accesibilidad, IME y paquetes multiplataforma.
- Integraciones bajo `integrations/`, mediante herramientas y protocolos
  oficiales; las interfaces no contienen políticas de trust o procesos.
- Supervisor local bajo demanda como propuesta para compartir procesos entre
  interfaces. Su IPC, exclusión mutua, crash/reconexión y políticas de cierre
  deben validarse antes de trasladar la autoridad de procesos.
- Una autoridad por operación durante la transición. Conservar contratos
  compatibles del bridge v1 y comparar mutaciones solo en copias aisladas.
- Rust se desarrolla inicialmente en `migration/rust/`; promoverlo a raíz al
  retirar el legado evita colisiones con `src/` Python. Un solo ciclo de release,
  manifiesto y política de actualización para todos los modos del producto.
- Preservar datos, backups, dry-run, permisos, idempotencia, canales y rollback.
  No importar confianza anterior si cambian su hash o semántica.
- Reemplazar Python/Qt únicamente tras paridad funcional, QA de cada plataforma
  e instaladores probados. No introducir LSP u otras ampliaciones como requisito
  implícito de esta migración.

## Consecuencias y límites

El costo principal aún no medido es reemplazar Scintilla y el render VT. Una GUI
Rust sin WebView no garantiza widgets del sistema, accesibilidad ni menor RAM.
La fase de viabilidad debe resolver estos riesgos antes del port completo.

Los ADR 0008, 0009, 0011, 0012 y 0013 conservan sus requisitos funcionales y de
seguridad; sus detalles Python/Qt/CMake se sustituyen por dominio cuando haya
evidencia de paridad. No se cambia retroactivamente su estado en esta entrega.

El producto continúa siendo un orquestador local. Este ADR no aprueba un
framework de agentes, un backend remoto ni cambios globales en el equipo.
