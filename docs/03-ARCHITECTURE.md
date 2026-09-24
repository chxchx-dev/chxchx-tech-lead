# 03 — Arquitectura

## Objetivo

`chichan-tech-lead` es la capa de automatización. No debe poseer toda la inteligencia del entorno.

```text
                  chichan-tech-lead
                         │
           ┌─────────────┼─────────────┐
           │             │             │
        Installer     Project       Doctor
                       Init
           │             │
           ▼             ▼
          uv         AGENTS.md
           │         CLAUDE.md
      ┌────┴────┐       .ai/
      │         │
Basic Memory  Serena
      │         │
      └────┬────┘
           │ MCP
    ┌──────┼────────┐
    │      │        │
 Claude  Codex   OpenCode
```

## Responsabilidades

### Basic Memory

Conocimiento persistente del proyecto y decisiones recuperables entre sesiones/agentes.

### Serena

Exploración semántica del código: símbolos, referencias y estructura.

### AGENTS.md

Constitución compacta del repositorio: reglas estables y expectativas de calidad.

### `.ai/`

Estado humano y operativo del proyecto, no una base de datos paralela.

### chichan-tech-lead

Instala, detecta, genera, sincroniza, diagnostica y configura adaptadores.

## Cadena de orientación de la IA

La preparación de un proyecto separa reglas, contexto y memoria para que cada agente pueda reconstruir el criterio de trabajo:

```text
Solicitud del usuario
        ↓
AGENTS.md / CLAUDE.md        reglas estables y específicas del cliente
        ↓
docs/                         arquitectura, seguridad y desarrollo
        ↓
.ai/                          proyecto, estado, roadmap y handoff
        ↓
Basic Memory + Serena          decisiones persistentes y navegación del código
        ↓
Cambio verificable             pruebas, smoke test y reporte de pendientes
```

`chichan-tech-lead` genera y sincroniza esta estructura, pero no decide el producto por el agente. La decisión sigue perteneciendo al usuario y al contexto del proyecto.

## Capas del código

```text
src/chichan_tech_lead/
├── cli.py
├── core/
│   ├── detector.py
│   ├── models.py
│   ├── managed.py
│   ├── registry.py
│   ├── backup.py
│   ├── runner.py
│   └── templates.py
└── integrations/
    ├── base.py
    ├── tools.py
    ├── installers.py
    └── mcp.py
```

## Regla arquitectónica

Una integración externa nunca debe filtrarse por todo el código. Si Claude/Codex/Serena cambian un comando, debe bastar con modificar el adapter correspondiente.

## Estado global

```text
~/.chichan-tech-lead/
├── projects.json
├── backups/
├── logs/
└── profiles/
```

No guardes credenciales aquí.
