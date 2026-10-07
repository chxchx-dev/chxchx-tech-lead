# chichan-tech-lead

## Propósito

Control plane local y liviano para preparar y operar repositorios de desarrollo asistido por agentes: workspace, procesos, terminales, CLI/TUI, skills, memoria y ChxChx Studio, la aplicación de escritorio nativa C++20/Qt 6 que comparte operaciones mediante el bridge JSON versionado.

## Stack detectado

- Perfil: `python`
- Stacks: python
- Lenguajes: python

## Restricciones

- Mantener CLI y TUI funcionales mientras se desarrolla el editor de escritorio.
- El editor debe llegar a paridad con las funciones y confirmaciones de la TUI.
- Distribución objetivo: macOS, Windows, Fedora y Ubuntu; interfaz C++ nativa, sin Chromium/WebEngine.
- No duplicar reglas de trust, recursos, skills o procesos entre las interfaces.
- Los documentos de trabajo de `docs/` se conservan solo en el checkout del mantenedor y están excluidos de Git. Las instrucciones requeridas para otros equipos están en `AGENTS.md`, `README.md` y `.ai/`.
