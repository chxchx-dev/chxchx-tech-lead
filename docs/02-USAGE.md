# 02 — Uso

## `chichan install`

Instala las dependencias MCP base que este proyecto administra actualmente.

```bash
chichan install --dry-run
chichan install
```

No instala Claude Code, Codex ni OpenCode. Esos clientes tienen ciclos de instalación distintos y el CLI solo los detecta/configura.

## `chichan init`

Inicializa un repositorio de forma idempotente.

```bash
chichan init
chichan init /ruta/a/otro/proyecto
```

Opciones importantes:

```bash
chichan init --dry-run
chichan init --no-backup
```

La operación crea un backup previo de `AGENTS.md`, `CLAUDE.md` y `.ai/` cuando existen.

## `chichan sync`

Regenera bloques marcados con:

```html
<!-- chichan:start project-rules -->
...
<!-- chichan:end project-rules -->
```

El contenido escrito manualmente fuera de estos bloques no se reemplaza.

## `chichan integrate`

Automatiza Claude/Codex mediante sus comandos MCP. Usa siempre `--dry-run` la primera vez en una máquina nueva.

```bash
chichan integrate --dry-run
chichan integrate --client claude
```

## `chichan doctor`

No intenta arreglar nada. Te muestra qué está instalado y qué falta.

## `chichan rollback`

Restaura el último backup local del proyecto administrado por Chichan.

```bash
chichan rollback
```

## Variable útil para pruebas

Puedes aislar el estado global del CLI con:

```bash
CHICHAN_HOME=/tmp/chichan-test chichan init --dry-run
```

En PowerShell:

```powershell
$env:CHICHAN_HOME="$env:TEMP\chichan-test"
chichan init --dry-run
```

## `chichan projects`

El registro global permite operar varios repositorios sin entrar manualmente en cada uno:

```bash
chichan projects list
chichan projects sync --dry-run
chichan projects sync
```

`projects sync` solo actualiza bloques administrados en `AGENTS.md` y `CLAUDE.md`; crea backup antes de cambios y omite rutas que ya no existen.

## Perfiles TOML

Los perfiles base viven en el paquete. Puedes añadir o sobrescribir perfiles propios en:

```text
~/.chichan-tech-lead/profiles/*.toml
```

Ejemplo:

```toml
[profiles.mi-stack]
stacks = ["nextjs", "nestjs"]
priority = 100
rules = ["Mantén los contratos de API versionados."]
```
