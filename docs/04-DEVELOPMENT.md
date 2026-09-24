# 04 — Desarrollo local

## Entorno

El proyecto usa Python y `uv`.

```bash
uv sync --dev
```

Para instalar el comando localmente en modo editable:

```bash
uv tool install --editable .
```

## Verificaciones antes de publicar

Ejecuta desde la raíz:

```bash
uv run python -m compileall -q src tests scripts
uv run pytest -q
uv run python scripts/lab_smoke.py
uv lock --check
```

La suite cubre registro, perfiles, backups, rollback, detección de stacks, sincronización, clientes MCP y comandos CLI. El smoke test usa un entorno temporal y no debe modificar la configuración del usuario.

## Reglas de implementación

- Mantener las operaciones idempotentes.
- Ofrecer `--dry-run` antes de cambios de configuración.
- No guardar secretos ni tokens.
- Encapsular cambios de integraciones en `integrations/`.
- Mantener compatibilidad con Linux, macOS y Windows cuando se use solo Python/`uv`.
- Toda operación destructiva debe tener backup o rollback claro.

Consulta [03-ARCHITECTURE.md](03-ARCHITECTURE.md) antes de cambiar límites entre módulos.
