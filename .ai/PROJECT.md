# chichan-tech-lead

## Propósito

Control plane local y liviano para preparar y operar repositorios de desarrollo asistido por agentes: workspace, procesos, terminales, CLI/TUI, skills, memoria y editores. La app de escritorio se construirá en C++ nativo y compartirá las mismas operaciones y estado que la CLI y la TUI.

## Stack detectado

- Perfil: `python`
- Stacks: python
- Lenguajes: python

## Restricciones

- Mantener CLI y TUI funcionales mientras se desarrolla el editor de escritorio.
- El editor debe llegar a paridad con las funciones y confirmaciones de la TUI.
- Distribución objetivo: macOS, Windows, Fedora y Ubuntu; interfaz C++ nativa, sin Chromium/WebEngine.
- No duplicar reglas de trust, recursos, skills o procesos entre las interfaces.
- Los documentos de `docs/` se conservan localmente y están excluidos de Git por decisión del usuario.
