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

### Skill Registry

La biblioteca local guarda instrucciones pequeñas en Markdown y metadatos TOML. El detector recomienda capacidades según archivos, dependencias, stacks y lenguajes; una recomendación nunca instala contexto por sí sola. El CLI y la pestaña Skills de la TUI operan el mismo registro y selección. Cada proyecto mantiene su selección en `.ai/chxchx-skills.toml`, y `skill sync` compone únicamente las instrucciones habilitadas en `.ai/SKILLS.md`. `AGENTS.md` recibe una referencia administrada para que los harnesses consulten ese contexto. La selección no reemplaza la memoria persistente de Basic Memory ni ejecuta workflows.

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

La TUI presenta estado y solicita acciones a los servicios de workspace. No inicia subprocess directamente. La CLI funciona de forma independiente a la TUI. CLI y TUI comparten tokens de color para que estados, confirmaciones y navegación mantengan la identidad visual.

Sublime sigue siendo un editor externo. `editor setup` genera, con `--dry-run` disponible, un `.sublime-project` local bajo `.ai/sublime/`, con exclusiones de artefactos comunes. Solo actualiza archivos con marca de administración ChxChx, conserva backup antes de actualizarlos y no configura preferencias globales del usuario. Las sesiones y procesos siguen bajo los servicios del workspace, no bajo el editor.

El CLI también opera el Skill Registry. Las mutaciones de selección y contexto aceptan `--dry-run`; la sincronización actualiza bloques administrados y guarda backup antes de escribir.

Studio consulta Handoff y Memoria mediante subcomandos JSON de solo lectura. La memoria usa `list_memory_notes`, filtra por título/contenido y rechaza rutas resueltas fuera del proyecto; el handoff se actualiza mediante el caso de uso `update_handoff`, con vista previa y confirmación en la interfaz.

Chats y Errores también son lecturas locales: `bridge chats` devuelve solo metadatos y vista previa, `bridge conversation` carga los mensajes de una sesión elegida, y `bridge errors` limita la caché a la ruta exacta del proyecto. Estas consultas se ejecutan fuera del hilo de UI y no modifican sesiones ni errores.

`ResourceManager` lee memoria, swap y CPU, clasifica umbrales y calcula avisos
antes de iniciar agentes desde la TUI y el CLI. `workspace.resources.max_agents`
limita la cantidad proyectada por proyecto; la TUI ofrece continuar/cancelar y
el CLI requiere `--force`. El RAM Governor no detiene procesos automáticamente.

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
