# chichan-tech-lead

## Propósito

Control plane local y liviano para preparar y operar repositorios de desarrollo asistido por agentes: workspace, procesos, terminales, CLI/TUI, skills, memoria y ChxChx Studio. La aplicación es un paquete Rust con CLI, TUI y Studio nativo en Iced. `old-version/` es solo una fuente de migración y debe quedar fuera del estado final. Si una capacidad exige Python o C++ sin alternativa viable, su código se aísla como servicio en `services/<nombre>/`, con protocolo y ciclo de vida definidos, pruebas y empaquetado, y se vincula a Rust mediante `src/integrations/`. Studio carga modelos y niveles de esfuerzo, persiste el historial visible, verifica sesiones restauradas y reconstruye un recap cuando el proveedor perdió una sesión. Rust normaliza eventos JSONL de Claude, Codex y OpenCode no-live, incluidas tarjetas live de Claude; el adapter aún conserva la ejecución y el control live mientras se completa el port.

## Stack detectado

- Principal: Rust (CLI, TUI y Studio)
- Servicios auxiliares Python/C++: solo cuando una dependencia los haga indispensables, aislados bajo `services/` y vinculados desde Rust.
- Plataformas objetivo: Windows, macOS, Fedora y Ubuntu

## Restricciones

La aplicación principal ya está migrada a Rust. El paquete mantiene una sola
versión, configuración y autoridad de casos de uso; las presentaciones son
módulos `cli`, `tui` y `studio`. Studio no usa WebView. Ver ADR 0016 y
[ROADMAP.md](ROADMAP.md) para capacidades pendientes de portar desde el ayudante.

[ROADMAP.md](ROADMAP.md) define ejecución y
[RELEASE-ROADMAP.md](RELEASE-ROADMAP.md) la secuencia de versiones y mantenimiento.

- Mantener CLI y TUI funcionales mientras se desarrolla el editor de escritorio.
- El editor debe llegar a paridad con las funciones y confirmaciones de la TUI.
- Distribución objetivo: macOS, Windows, Fedora y Ubuntu; Studio nativo, sin Chromium/WebEngine.
- Los instaladores gráficos de macOS y Linux y el MSI/ZIP de Windows se publican como assets de GitHub Releases con SHA-256 en el manifiesto; se instalan por usuario y agregan los comandos CLI empaquetados al PATH.
- No duplicar reglas de trust, recursos, skills o procesos entre las interfaces.
- Los documentos de trabajo de `docs/` se conservan solo en el checkout del mantenedor y están excluidos de Git. Las instrucciones requeridas para otros equipos están en `AGENTS.md`, `README.md` y `.ai/`.
