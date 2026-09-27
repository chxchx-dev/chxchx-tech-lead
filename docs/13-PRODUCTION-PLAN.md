# 13 — Rollout de `v0.2.0`

`v0.2.0` es una release estable para uso propio. Este documento describe cómo incorporarla gradualmente a proyectos reales; las tareas ya implementadas están resumidas en [CHANGELOG.md](../CHANGELOG.md).

## Alcance entregado

- Preparación idempotente de proyectos con perfiles y detección de stack.
- Backups, `--dry-run` y rollback para cambios gestionados.
- Instalación de Basic Memory y Serena mediante `uv`.
- Integración MCP con Claude, Codex y OpenCode.
- Registro y sincronización de múltiples proyectos.
- Diagnóstico del entorno y smoke test aislado.

## Evidencia local de la release

Desde el repositorio:

```bash
uv run python -m compileall -q src tests scripts
uv run pytest -q
uv run python scripts/lab_smoke.py
uv lock --check
```

La suite actual debe pasar completa. El smoke test debe terminar con `lab smoke: ok`.

## Rollout recomendado

1. Instala la etiqueta `v0.2.0` o usa una instalación editable si estás desarrollando.
2. Ejecuta `chxchx-tech doctor` y corrige solo los requisitos que realmente vayas a usar.
3. En un proyecto laboratorio, ejecuta `chxchx-tech setup --dry-run`, revisa el plan y luego `chxchx-tech setup`.
4. Conecta un único cliente MCP usando primero `chxchx-tech integrate --dry-run`.
5. Valida el trabajo diario del proyecto laboratorio y conserva el backup inicial.
6. Repite el proceso en los proyectos grandes, uno por uno.

## Recuperación

Antes de aceptar cambios en un proyecto real, revisa `chxchx-tech status`. Si necesitas deshacer la última modificación gestionada:

```bash
chxchx-tech rollback
```

El rollback restaura el backup más reciente y conserva el estado actual según el mecanismo de backup de la operación. Si el cambio pertenece a un archivo fuera del alcance gestionado, recupéralo desde el control de versiones del propio proyecto.

## Próxima revisión

Después de varias semanas de uso real, registra problemas y necesidades nuevas. Solo entonces decide si merece la pena abordar los elementos futuros de [05-ROADMAP.md](05-ROADMAP.md).
