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
