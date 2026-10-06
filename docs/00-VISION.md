# 00 — Visión

## Problema

Cada repositorio nuevo repite la misma preparación: reglas para agentes, memoria, herramientas MCP, documentación operativa y configuración de clientes.

## Solución

`chxchx-tech-lead` no crea otro agente ni otra memoria. Es el bootstrapper/orquestador que conecta componentes especializados:

```text
ChxChx
  │
  ├─ Claude Code
  ├─ Codex
  └─ OpenCode
        │
        └─ MCP
           ├─ Basic Memory → conocimiento persistente en Markdown
           └─ Serena       → comprensión semántica del código

Repositorio
  ├─ AGENTS.md     → reglas estables
  ├─ CLAUDE.md     → instrucciones específicas de Claude
  ├─ .ai/          → estado, roadmap, handoff y memoria del proyecto
  └─ docs/         → arquitectura, ADR, seguridad, BD y API
```

## Resultado deseado

Después de instalar el CLI una vez, el flujo ideal es:

```bash
cd mi-proyecto
chxchx-tech setup --dry-run
chxchx-tech setup
```

El segundo comando debe dejar preparado lo necesario para comenzar a trabajar con los agentes disponibles en la máquina.

## Qué NO es

- No es un reemplazo de Claude/Codex/OpenCode.
- No es un reemplazo de Basic Memory.
- No es un reemplazo de Serena.
- No es DevBrain todavía.
- No debe convertirse en un framework enorme sin presión real de uso.

## Cuándo construir DevBrain

Solo después de usar este flujo en proyectos reales y encontrar necesidades que la combinación actual no cubra: claims de tareas, handoffs estructurados, actividad de agentes, fallos/soluciones o coordinación multiagente más fuerte.
