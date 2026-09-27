# 02 — Uso

## `chxchx-tech install`

Instala las dependencias MCP base que este proyecto administra actualmente.

```bash
chxchx-tech install --dry-run
chxchx-tech install
```

No instala Claude Code, Codex ni OpenCode. Esos clientes tienen ciclos de instalación distintos y el CLI solo los detecta/configura.

## `chxchx-tech init`

Inicializa un repositorio de forma idempotente.

```bash
chxchx-tech init
chxchx-tech init /ruta/a/otro/proyecto
```

Opciones importantes:

```bash
chxchx-tech init --dry-run
chxchx-tech init --no-backup
```

La operación crea un backup previo de `AGENTS.md`, `CLAUDE.md` y `.ai/` cuando existen.

## `chxchx-tech sync`

Regenera bloques marcados con:

```html
<!-- chxchx-tech:start project-rules -->
...
<!-- chxchx-tech:end project-rules -->
```

El contenido escrito manualmente fuera de estos bloques no se reemplaza.

## `chxchx-tech integrate`

Automatiza Claude/Codex mediante sus comandos MCP. Usa siempre `--dry-run` la primera vez en una máquina nueva.

```bash
chxchx-tech integrate --dry-run
chxchx-tech integrate --client claude
```

## `chxchx-tech doctor`

No intenta arreglar nada. Te muestra qué está instalado y qué falta.

## `chxchx-tech rollback`

Restaura el último backup local del proyecto administrado por ChxChx.

```bash
chxchx-tech rollback
```

## Variable útil para pruebas

Puedes aislar el estado global del CLI con:

```bash
CHXCHX_TECH_HOME=/tmp/chxchx-tech-test chxchx-tech init --dry-run
```

En PowerShell:

```powershell
$env:CHXCHX_TECH_HOME="$env:TEMP\chxchx-tech-test"
chxchx-tech init --dry-run
```

## `chxchx-tech projects`

El registro global permite operar varios repositorios sin entrar manualmente en cada uno:

```bash
chxchx-tech projects list
chxchx-tech projects sync --dry-run
chxchx-tech projects sync
```

`projects sync` solo actualiza bloques administrados en `AGENTS.md` y `CLAUDE.md`; crea backup antes de cambios y omite rutas que ya no existen.

## Perfiles TOML

Los perfiles base viven en el paquete. Puedes añadir o sobrescribir perfiles propios en:

```text
~/.chxchx-tech-lead/profiles/*.toml
```

Ejemplo:

```toml
[profiles.mi-stack]
stacks = ["nextjs", "nestjs"]
priority = 100
rules = ["Mantén los contratos de API versionados."]
```
