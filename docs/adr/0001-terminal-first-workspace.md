# ADR-0001: Terminal-first workspace con herramientas externas

- Estado: Aceptada
- Fecha: 2026-09-28
- Alcance: arquitectura de Terminal Workspace para `chxchx-tech-lead`

## Contexto

El bootstrapper prepara repositorios con reglas, memoria e integraciones. El trabajo diario en varios repositorios también requiere administrar procesos, terminales, sesiones, agentes y consumo de recursos. La máquina objetivo tiene recursos limitados y ya existen herramientas especializadas para esas funciones.

La evolución debe conservar los comandos de preparación de proyectos y evitar ejecutar comandos de un repositorio no confiable.

## Decisión

ChxChx será un control plane local terminal-first:

- El CLI y la TUI presentan acciones y estado; no contienen la lógica de ejecución de procesos.
- `core/` administra detección, configuración, confianza, estado y migraciones.
- `workspace/` coordina sesiones, procesos, recursos, agentes y handoff.
- `adapters/` aísla Zellij, subprocess, Sublime, Git, Docker Compose y las CLI de agentes.
- `integrations/` mantiene las integraciones de instalación y MCP existentes.
- `setup` e `init` preparan configuración; las operaciones de workspace ejecutan procesos.
- Los comandos se pasan como argumentos separados por defecto. El uso de shell debe declararse de forma explícita.
- Los proyectos requieren confianza local antes de ejecutar procesos configurados.
- El estado y los logs viven fuera del repositorio preparado.

ChxChx no implementará un emulador de terminal, editor, cliente Git, runtime propio de agentes ni daemon de Docker.

## Consecuencias

- Las herramientas externas conservan sus ciclos de instalación y actualización.
- Los adapters pueden sustituirse o ampliarse sin filtrar comandos de terceros al dominio.
- La CLI sigue operativa sin TUI y sin Zellij; algunas funciones quedan limitadas al adapter disponible.
- Persistencia atómica, logs acotados, validación de rutas y control de PID reducen riesgos de pérdida de estado y de afectar procesos ajenos.
- Linux es la plataforma principal; el soporte de macOS y Windows/WSL debe validarse en cada sistema antes de declarar `v1.0`.
- No se publica `v1.0` hasta contar con evidencia de uso diario, recuperación y consumo en proyectos reales.

## Alternativas descartadas

- Crear un editor o terminal propios: duplicaría herramientas maduras y elevaría el consumo.
- Ejecutar procesos desde la TUI: mezcla presentación y efectos del sistema, además de complicar pruebas.
- Arrancar comandos automáticamente al preparar un repo: permite que un clon ejecute código sin aprobación local.
- Coordinar agentes autónomos sobre el mismo árbol: aumenta el riesgo de cambios concurrentes difíciles de controlar.
