# 05 — Roadmap posterior a `v0.2.0`

`v0.2.0` es la base estable para uso propio. El historial de lo que ya se implementó está en [CHANGELOG.md](../CHANGELOG.md); este documento solo conserva trabajo futuro.

## v0.3 — Integraciones opcionales

- Añadir integraciones opcionales para GitHub CLI y Obsidian si aportan valor real.
- Mejorar diagnósticos específicos por cliente MCP.
- Automatizar reparaciones únicamente cuando sean seguras, reversibles y solicitadas explícitamente.

## v0.4 — Operación multi-proyecto

- Vista agregada del estado de todos los proyectos registrados.
- Migraciones explícitas para plantillas `.ai/` existentes.
- Flujo seguro para desregistrar proyectos, con confirmación y backup.

## v1.0 — Validación amplia

- Uso diario en varios proyectos reales.
- Verificación continua en Linux, macOS y Windows.
- Documentación operativa de recuperación y mantenimiento.
- Revisión de compatibilidad con nuevas versiones de `uv`, Python y clientes MCP.

## Fuera de alcance por ahora

No convertir el proyecto en un framework de agentes ni añadir un servidor central mientras el uso local no demuestre una necesidad concreta.
