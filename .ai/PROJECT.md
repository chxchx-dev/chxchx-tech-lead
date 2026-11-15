# chichan-tech-lead

## Propósito

Control plane local y liviano para preparar y operar repositorios de desarrollo asistido por agentes: workspace, procesos, terminales, CLI/TUI, skills, memoria y ChxChx Studio, la aplicación de escritorio nativa C++20/Qt 6 que comparte operaciones mediante el bridge JSON versionado.

## Stack detectado

- Perfil: `python`
- Stacks: python
- Lenguajes: python

## Restricciones

Dirección futura solicitada el 2026-11-12: núcleo e interfaces en Rust, Studio
nativo sin WebView. Ver [RUST-MIGRATION.md](RUST-MIGRATION.md) y ADR 0016
(propuesto). La implementación y las restricciones del stack actual descritas
abajo se conservan durante la transición; no fijan el toolkit Rust definitivo.

Organización futura aclarada: una sola aplicación/paquete Rust por capacidades,
con CLI, TUI y Studio como módulos de presentación; no proyectos ni crates por
interfaz. Misma configuración, casos de uso, versión y ciclo de actualización.
[ROADMAP.md](ROADMAP.md) define ejecución y
[RELEASE-ROADMAP.md](RELEASE-ROADMAP.md) la secuencia de versiones y mantenimiento.

- Mantener CLI y TUI funcionales mientras se desarrolla el editor de escritorio.
- El editor debe llegar a paridad con las funciones y confirmaciones de la TUI.
- Distribución objetivo: macOS, Windows, Fedora y Ubuntu; interfaz C++ nativa, sin Chromium/WebEngine.
- No duplicar reglas de trust, recursos, skills o procesos entre las interfaces.
- Los documentos de trabajo de `docs/` se conservan solo en el checkout del mantenedor y están excluidos de Git. Las instrucciones requeridas para otros equipos están en `AGENTS.md`, `README.md` y `.ai/`.
