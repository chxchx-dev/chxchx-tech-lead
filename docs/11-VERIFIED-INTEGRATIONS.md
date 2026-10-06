# 11 — Integraciones verificadas

Última revisión documental: **2026-09-23**.

Este archivo existe porque las CLI externas cambian. Cuando una integración falle, revisa primero la documentación oficial y modifica únicamente `src/chxchx_tech_lead/integrations/`.

## Basic Memory

Instalación local cross-platform:

```bash
uv tool install basic-memory
```

MCP local:

```bash
basic-memory mcp
```

Proyecto local:

```bash
basic-memory project add NOMBRE /ruta/al/directorio
```

Fuentes:

- https://docs.basicmemory.com/start-here/quickstart-local
- https://docs.basicmemory.com/reference/cli-reference
- https://docs.basicmemory.com/integrations/claude-code/
- https://docs.basicmemory.com/integrations/codex/

## Serena

Instalación recomendada:

```bash
uv tool install -p 3.13 serena-agent
```

Servidor MCP instalado:

```bash
serena start-mcp-server --context ide-assistant --project-from-cwd
```

Fuentes:

- https://github.com/oraios/serena/blob/main/docs/02-usage/010_installation.md
- https://github.com/oraios/serena/blob/main/docs/02-usage/020_running.md

## Claude Code

Claude Code permite registrar MCP mediante:

```bash
claude mcp add NOMBRE -- COMANDO ARGUMENTOS...
```

Basic Memory documenta específicamente:

```bash
claude mcp add basic-memory basic-memory mcp
```

Fuente:

- https://docs.basicmemory.com/integrations/claude-code/

## Codex CLI

Codex comparte la configuración MCP entre CLI/IDE y permite registrar servidores con `codex mcp add`.

Fuente:

- https://developers.openai.com/learn/docs-mcp
- https://docs.basicmemory.com/integrations/codex/

## OpenCode

OpenCode v2 admite:

```bash
opencode mcp add NOMBRE -- COMANDO ARGUMENTOS...
```

Su configuración actual usa `mcp.servers` para servidores locales.

Fuente:

- https://opencode.ai/v2/docs/mcp-servers
## Compatibilidad observada

Algunas versiones de Basic Memory pueden devolver error al intentar añadir un proyecto que ya existe, aunque el proyecto quede disponible. El adapter confirma `project info` después de un `add` fallido y trata el caso como estado idempotente cuando la confirmación es positiva.
