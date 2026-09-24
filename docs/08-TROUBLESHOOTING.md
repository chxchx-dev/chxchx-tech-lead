# 08 — Troubleshooting

## `chichan: command not found`

Comprueba que la herramienta esté instalada y que el directorio de herramientas de `uv` esté en `PATH`:

```bash
uv tool list
uv tool update-shell
```

Después abre una terminal nueva. Para instalar la release estable desde Git:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chichan-tech-lead.git@v0.2.0"
```

## `uv` no encontrado

Instálalo usando el método oficial de tu sistema y abre una terminal nueva. Los instaladores están en [README.md](../README.md) y [01-QUICKSTART.md](01-QUICKSTART.md).

## Basic Memory o Serena no aparecen después de instalar

```bash
chichan doctor
uv tool list
```

Si `uv` los muestra pero el executable no está en `PATH`, ejecuta:

```bash
uv tool update-shell
```

Luego abre una terminal nueva.

## La integración MCP falla

Primero limita la operación a un cliente y revisa el plan:

```bash
chichan integrate --dry-run --client claude
```

Después prueba manualmente el comando que muestra. Las CLI externas cambian con el tiempo; el adapter `integrations/mcp.py` es el único lugar que debería requerir ajuste.

## Quiero deshacer cambios del init

```bash
chichan rollback
```

El rollback cubre archivos administrados respaldados previamente; no es un reemplazo para Git.
