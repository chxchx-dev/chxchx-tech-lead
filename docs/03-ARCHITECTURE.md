# 03 — Arquitectura

## Objetivo

`chxchx-tech-lead` es la capa de automatización. No debe poseer toda la inteligencia del entorno.

```text
                  chxchx-tech-lead
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

### chxchx-tech-lead

Instala, detecta, genera, sincroniza y diagnostica. El CLI y la TUI operan el workspace mediante servicios de dominio y adapters aislados.

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

`chxchx-tech-lead` genera y sincroniza esta estructura, pero no decide el producto por el agente. La decisión sigue perteneciendo al usuario y al contexto del proyecto.

## Capas del código

~~~text
src/chxchx_tech_lead/
├── cli.py             # punto de entrada público
├── cli_context.py     # registro Typer y presentación compartida
├── commands/          # setup, workspace, agentes, procesos, proyectos e integraciones
├── core/             configuración, detección, trust, migraciones y estado base
├── workspace/        fachada y casos de uso para agentes/procesos, modelos y estado
├── adapters/         terminal, editor, agentes y Git
├── integrations/     Basic Memory, Serena, instaladores y MCP
└── tui/              interfaz Textual
~~~

Dirección de dependencias:

~~~text
CLI / TUI ──> core / workspace ──> adapters ──> herramientas externas
                    │
                    └────────────> integrations
~~~

La TUI presenta estado y solicita acciones a los servicios de workspace. No inicia subprocess directamente. La CLI funciona de forma independiente a la TUI.

## Regla arquitectónica

Una integración externa nunca debe filtrarse por todo el código. Si Claude/Codex/Serena cambian un comando, debe bastar con modificar el adapter correspondiente.

## Estado global

```text
~/.chxchx-tech-lead/
├── projects.json
├── trusted_projects.json
├── workspace-state.json
├── backups/
├── logs/workspaces/
└── profiles/
```

No guardes credenciales aquí.

## Registro de decisiones

- [ADR-0001: Terminal-first workspace con herramientas externas](adr/0001-terminal-first-workspace.md)
