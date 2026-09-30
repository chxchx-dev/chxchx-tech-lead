# 05 — Roadmap hacia `v1.0`

La base publicada `v0.2.0` sigue estable. En la rama `dev`, las fases 1–7 del plan de Terminal Workspace ya están implementadas. El trabajo activo es cerrar hardening y validar el uso real antes de declarar `v1.0`; el detalle técnico está en [14-TERMINAL-WORKSPACE-PLAN.md](14-TERMINAL-WORKSPACE-PLAN.md).

## Estado actual

- Dominio, configuración v2, confianza, adapters, CLI, TUI, recursos, multiproyecto y agentes: implementados.
- Persistencia atómica del estado y rotación acotada de logs: implementadas y cubiertas por pruebas.
- Hardening de procesos y migración de config v1 → v2: implementados.
- Prueba manual reportada como exitosa por el usuario en macOS y Windows 11 Pro nativo (30-09-2026).
- Las pruebas cubren la regresión CLI heredada, reanudación reutilizando una sesión, salida segura de la TUI y rechazo de PIDs ajenos.
- Se endureció la terminación de árboles Windows; las pruebas manuales de Windows 11 Pro y macOS quedan reservadas para cuando la versión esté más avanzada, como indicó el usuario.
- WSL no está validado; falta confirmar CI verde y documentar los escenarios manuales ejercitados.
- Evidencia prolongada de uso diario: pendiente.

## Siguiente: hardening multiplataforma

1. Confirmar y registrar una ejecución verde de CI en Linux, macOS y Windows (Python 3.11–3.13); el workflow ya contiene esa matriz.
2. Completar el registro de la prueba manual exitosa en macOS y Windows 11 Pro nativo: anotar los escenarios ejercitados; WSL sigue pendiente si se mantiene en el alcance.
3. Al acercarse a la release, revalidar en Windows 11 Pro la terminación de procesos con hijos y en macOS las sesiones/shells; comprobar recuperación tras cierres inesperados. Probar WSL si permanece en el alcance.
4. Medir consumo y estabilidad con proyectos reales durante uso diario y documentar resultados/incidentes.
5. Mantener las guías de recuperación y compatibilidad del CLI tradicional.

## Criterios para `v1.0`

- Uso diario sostenido en proyectos reales.
- No detener procesos ajenos y recuperar sesiones correctamente.
- Migraciones seguras y CLI tradicional estable.
- La TUI puede cerrarse sin detener el workspace.
- Consumo propio bajo y resultados de CI y pruebas manuales documentados en Linux, macOS y Windows 11 Pro; validar WSL si se mantiene como plataforma objetivo.
- Documentación operativa de recuperación y mantenimiento.

La experiencia diaria, el consumo y la estabilidad en proyectos reales requieren evidencia del entorno de uso; no se consideran completados solo porque pase la suite local.

## Ideas opcionales, fuera del cierre de `v1.0`

- Integraciones con GitHub CLI u Obsidian si existe una necesidad concreta.
- Diagnósticos MCP más específicos.
- Reparaciones asistidas, siempre explícitas, reversibles y seguras.

No convertir el proyecto en un framework de agentes ni añadir un servidor central sin una necesidad demostrada.
