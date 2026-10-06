# 08 — Troubleshooting

## `chxchx-tech: command not found`

Comprueba que la herramienta esté instalada y que el directorio de herramientas de `uv` esté en `PATH`:

```bash
uv tool list
uv tool update-shell
```

Después abre una terminal nueva. Para instalar la release estable desde Git:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
```

## `uv` no encontrado

Instálalo usando el método oficial de tu sistema y abre una terminal nueva. Los instaladores están en [README.md](../README.md) y [01-QUICKSTART.md](01-QUICKSTART.md).

## Quiero una instalación reducida

Usa el bootstrap remoto para instalar solo el ejecutable global:

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/main/scripts/bootstrap.sh | sh
```

Después, dentro de cada proyecto, usa `chxchx-tech init --minimal` para crear
solo la configuración `.ai/` y la memoria local.

## Basic Memory o Serena no aparecen después de instalar

```bash
chxchx-tech doctor
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
chxchx-tech integrate --dry-run --client claude
```

Después prueba manualmente el comando que muestra. Las CLI externas cambian con el tiempo; el adapter `integrations/mcp.py` es el único lugar que debería requerir ajuste.

## La pane `terminal` acepta comandos pero no muestra lo que escribo

Las sesiones nuevas de Zellij preparadas por ChxChx restauran automáticamente el
TTY antes de iniciar el shell. Si una sesión antigua conserva el estado de una
pane anterior, puedes repararla una vez dentro de la pane `terminal`:

```bash
stty sane
```

Para aplicar el layout actualizado a una sesión existente, ciérrala cuando no
tengas trabajo pendiente dentro de ella y vuelve a iniciarla desde el proyecto:

```bash
zellij delete-session --force NOMBRE_DEL_PROYECTO
cd /ruta/al/proyecto
chxchx-tech tui .
```

## Quiero deshacer cambios del init

```bash
chxchx-tech rollback
```

El rollback cubre archivos administrados respaldados previamente; no es un reemplazo para Git.
