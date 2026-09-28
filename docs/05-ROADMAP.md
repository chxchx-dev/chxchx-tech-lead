# 05 — Roadmap hacia `v1.0`

La base publicada `v0.2.0` sigue estable. En la rama `dev`, las fases 1–7 del plan de Terminal Workspace ya están implementadas. El trabajo activo es cerrar hardening y validar el uso real antes de declarar `v1.0`; el detalle técnico está en [14-TERMINAL-WORKSPACE-PLAN.md](14-TERMINAL-WORKSPACE-PLAN.md).

## Estado actual

- Dominio, configuración v2, confianza, adapters, CLI, TUI, recursos, multiproyecto y agentes: implementados.
- Persistencia atómica del estado y rotación acotada de logs: implementadas y cubiertas por pruebas.
- Hardening de procesos y migración de config v1 → v2: implementados.
- Validación nativa en Windows/macOS/Linux y evidencia prolongada de uso diario: pendientes.

## Siguiente: hardening multiplataforma

1. Ejecutar la suite en Linux, macOS y Windows/WSL; resolver cualquier diferencia detectada.
2. Confirmar señales, cierre de árboles de procesos, quoting y shells en cada plataforma.
3. Revisar recuperación de sesiones y estado tras cierres inesperados.
4. Medir consumo y estabilidad con proyectos reales.
5. Mantener las guías de recuperación y la compatibilidad del CLI tradicional.

## Criterios para `v1.0`

- Uso diario sostenido en proyectos reales.
- No detener procesos ajenos y recuperar sesiones correctamente.
- Migraciones seguras y CLI tradicional estable.
- La TUI puede cerrarse sin detener el workspace.
- Consumo propio bajo y validación Linux/macOS/WSL documentada.
- Documentación operativa de recuperación y mantenimiento.

La experiencia diaria, el consumo y la estabilidad en proyectos reales requieren evidencia del entorno de uso; no se consideran completados solo porque pase la suite local.

## Ideas opcionales, fuera del cierre de `v1.0`

- Integraciones con GitHub CLI u Obsidian si existe una necesidad concreta.
- Diagnósticos MCP más específicos.
- Reparaciones asistidas, siempre explícitas, reversibles y seguras.

No convertir el proyecto en un framework de agentes ni añadir un servidor central sin una necesidad demostrada.
