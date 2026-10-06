# 06 — Seguridad

## Reglas

- Nunca guardar API keys, tokens, cookies o credenciales en `.ai/`.
- No copiar `.env` a memoria persistente.
- No registrar salida sensible de comandos.
- No editar configuraciones de terceros sin backup cuando el cambio pueda romper la herramienta.
- Preferir comandos oficiales (`claude mcp`, `codex mcp`) a manipulación directa de archivos internos.
- Usar `--dry-run` antes de integraciones en una máquina nueva.

## MCP

Un servidor MCP tiene capacidad de exponer herramientas al agente. Instala únicamente MCP confiables y mantén el conjunto pequeño.

## Repositorios

`AGENTS.md`, `.ai/` y documentación pueden contener contexto interno del proyecto. Decide conscientemente qué repositorios serán públicos.

El `.gitignore` del bootstrapper excluye el contexto generado por ChxChx,
memorias Serena, bases de datos SQLite, logs, credenciales y material
criptográfico. Los archivos de documentación propios del repositorio siguen
versionados; en un proyecto preparado se recomienda revisar `git status` antes
del primer commit público.
