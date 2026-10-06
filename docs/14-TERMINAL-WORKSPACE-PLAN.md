# 14 — Plan maestro: ChxChx Terminal Workspace

> Documento de planificación y estado para evolucionar `chxchx-tech-lead` desde la base estable `v0.2.0` hacia un entorno de desarrollo **terminal-first**, ligero, multiproyecto y asistido por IA.
>
> Este documento describe **qué construir, en qué orden, qué no construir, criterios de aceptación, arquitectura, seguridad y estrategia de migración**. Las casillas de las fases reflejan el estado real del MVP implementado; lo que permanece abierto sigue siendo roadmap.

> **Estado de esta entrega:** las fases 1–7 están implementadas. El hardening local incluye persistencia atómica del estado, rotación acotada de logs, terminación de procesos por plataforma, argumentos seguros y migración versionada de config. El usuario reportó pruebas manuales exitosas en macOS y Windows 11 Pro nativo el 2026-09-30. El PR pasó la matriz CI de Linux, macOS y Windows con Python 3.11, 3.12 y 3.13 (9 combinaciones; resultado reportado por el usuario). La regresión CLI heredada y la salida segura de la TUI tienen cobertura automatizada. Quedan la evidencia de uso sostenido y la validación manual de release; WSL está pendiente de confirmar como plataforma objetivo.
>
> **Cambio de alcance:** la integración y gestión de Docker fue retirada. Las referencias a Docker/Compose en este plan son históricas y no forman parte de la herramienta actual ni deben reimplementarse.

---

## 1. Punto de partida

La versión actual del proyecto es `v0.2.0` y ya resuelve correctamente la capa de bootstrap/orquestación inicial:

- CLI con Typer + Rich.
- Detección de stacks y perfiles.
- Registro global de proyectos.
- `.ai/chxchx-tech.toml` por proyecto.
- Generación y sincronización de `AGENTS.md`, `CLAUDE.md` y `.ai/`.
- Basic Memory y Serena.
- Integración MCP con Claude, Codex y OpenCode.
- Backups, `--dry-run` y rollback.
- `doctor`, `status`, `setup`, `sync` y operación multiproyecto básica.

La siguiente evolución **no debe reemplazar esa base**. Debe usarla como núcleo y añadir una capa de workspace operativo.

---

## 2. Problema que debe resolver `v0.3+`

El problema real no es únicamente abrir un editor. En una máquina con recursos limitados, trabajar en varios repositorios implica normalmente mantener:

- varias ventanas del IDE;
- varios servidores de desarrollo;
- Docker/Compose;
- TypeScript Language Server y extensiones;
- varias terminales;
- Codex, Claude u OpenCode;
- Git;
- navegador;
- bases de datos locales.

El objetivo de ChxChx será reducir esa fragmentación y permitir trabajar así:

```text
Terminal real
   │
   └── chxchx-tech
          │
          ├── proyecto activo
          ├── procesos
          ├── servicios Docker
          ├── Git
          ├── agentes IA
          ├── consumo de recursos
          ├── sesión Zellij
          └── editor Sublime
```

El editor deja de ser el centro de operaciones. **ChxChx se convierte en el control plane local** y Sublime en un editor/visor ligero.

---

## 3. Principio rector

### ChxChx orquesta; no reimplementa

No construir:

- un emulador de terminal;
- un editor de código;
- un daemon de Docker;
- un cliente Git completo;
- un runtime de agentes propio;
- un reemplazo de Codex/Claude/OpenCode;
- un sistema de ventanas propio;
- un framework multiagente gigantesco.

Usar herramientas maduras mediante adaptadores:

```text
                      CHXCHX
                         │
        ┌────────────────┼────────────────┐
        │                │                │
      Editor          Workspace        Agents
        │                │                │
     Sublime           Zellij       Codex/Claude
        │                │             OpenCode
        │                │
        └──────┬─────────┴──────┬─────────┘
               │                │
              Git            Docker
```

La ventaja competitiva de ChxChx debe ser:

1. un solo punto de entrada;
2. contexto consistente por proyecto;
3. arranque y cierre controlado;
4. bajo consumo;
5. sesiones recuperables;
6. multiproyecto sin dejar todos los stacks corriendo;
7. agentes IA conectados al mismo contexto y reglas.

---

## 4. Objetivo del producto

El flujo final esperado será:

```bash
cd ~/Projects/acore-crm
chxchx-tech
```

Y ChxChx deberá poder mostrar algo equivalente a:

```text
╭──────────────────────────────────────────────────────────────╮
│ CHXCHX TECH LEAD                         ACore CRM           │
├──────────────────────────────────────────────────────────────┤
│ Stack     .NET · React · PostgreSQL · Redis                  │
│ Git       feature/storage-r2                                 │
│ Session   acore-crm                         ● ACTIVE          │
│ RAM       8.2 / 16.0 GB                     51%               │
├──────────────────────────────────────────────────────────────┤
│ Processes                                                    │
│ ● backend       dotnet watch             :5000    610 MB     │
│ ● frontend      npm run dev              :3000    720 MB     │
│ ● postgres      docker                    :5432    190 MB     │
│ ○ worker        stopped                                      │
├──────────────────────────────────────────────────────────────┤
│ Agents                                                       │
│ ○ Codex          ○ Claude          ○ OpenCode                │
├──────────────────────────────────────────────────────────────┤
│ [P] Projects [A] Agent [R] Run [G] Git [D] Docker [Q] Exit  │
╰──────────────────────────────────────────────────────────────╯
```

El usuario debe poder abrir Sublime, arrancar/parar procesos, iniciar un agente, cambiar de proyecto o adjuntarse a una sesión existente sin reconstruir manualmente el entorno.

---

# 5. Arquitectura objetivo

```text
src/chxchx_tech_lead/
├── __init__.py
├── cli.py
│
├── core/
│   ├── backup.py
│   ├── detector.py
│   ├── managed.py
│   ├── models.py
│   ├── paths.py
│   ├── profiles.py
│   ├── project_config.py
│   ├── registry.py
│   ├── runner.py
│   ├── sync.py
│   ├── templates.py
│   ├── trust.py                 # nuevo
│   └── config_migrations.py     # nuevo
│
├── workspace/                   # nuevo
│   ├── __init__.py
│   ├── manager.py
│   ├── models.py
│   ├── state.py
│   ├── process_manager.py
│   ├── resources.py
│   ├── health.py
│   ├── ports.py
│   └── layouts.py
│
├── adapters/                    # nuevo
│   ├── __init__.py
│   ├── editor/
│   │   ├── base.py
│   │   └── sublime.py
│   ├── terminal/
│   │   ├── base.py
│   │   ├── zellij.py
│   │   └── subprocess.py
│   ├── agents/
│   │   ├── base.py
│   │   ├── codex.py
│   │   ├── claude.py
│   │   └── opencode.py
│   ├── docker/
│   │   ├── base.py
│   │   └── compose.py
│   └── git/
│       ├── base.py
│       └── cli.py
│
├── tui/                         # nuevo
│   ├── __init__.py
│   ├── app.py
│   ├── styles.tcss
│   ├── screens/
│   │   ├── dashboard.py
│   │   ├── projects.py
│   │   ├── processes.py
│   │   ├── agents.py
│   │   ├── git.py
│   │   └── settings.py
│   └── widgets/
│       ├── header.py
│       ├── project_status.py
│       ├── process_table.py
│       ├── agent_status.py
│       ├── resource_meter.py
│       └── log_panel.py
│
└── integrations/
    ├── base.py
    ├── basic_memory.py
    ├── installers.py
    ├── mcp.py
    └── tools.py
```

## Regla de dependencias

```text
TUI ────────┐
CLI ────────┼──> workspace/core ───> adapters ───> herramientas externas
             │
             └──> integrations actuales
```

La TUI no debe ejecutar directamente `subprocess`. Toda acción debe pasar por servicios/adaptadores testeables.

---

# 6. Modelo de workspace

## Estados de un proyecto

Cada proyecto registrado podrá tener uno de estos estados operativos:

### `ACTIVE`

- sesión disponible;
- uno o más procesos ejecutándose;
- proyecto actualmente seleccionado;
- editor/agentes pueden estar abiertos.

### `SUSPENDED`

- sesión conservada cuando sea posible;
- procesos pesados detenidos o pausados según adapter;
- metadata y layout conservados;
- pensado para cambiar temporalmente a otro proyecto.

### `STOPPED`

- ningún proceso de desarrollo gestionado por ChxChx activo;
- se conserva la configuración del workspace;
- debe consumir prácticamente cero RAM adicional.

### `DEGRADED`

- workspace parcialmente funcional;
- ejemplo: frontend vivo pero Docker falló.

### `ERROR`

- fallo que impide usar el workspace como fue configurado;
- debe mostrar causa y comando de recuperación.

---

# 7. Configuración de proyecto `chxchx-tech.toml` v2

La configuración actual es mínima:

```toml
version = 1
profile = "..."
memory_project = "..."
memory_path = ".ai/memory"
```

No se debe romper. Se creará una migración explícita a `version = 2`.

## Esquema objetivo

```toml
version = 2
profile = "dotnet"
memory_project = "acore-crm-a1b2c3"
memory_path = ".ai/memory"

[workspace]
name = "acore-crm"
adapter = "zellij"
editor = "sublime"
auto_open_editor = false
auto_attach = true
auto_start = false

[workspace.resources]
warn_memory_percent = 75
critical_memory_percent = 90
warn_swap_percent = 40

[workspace.docker]
enabled = true
compose_file = "compose.yaml"
auto_start = false

[[workspace.processes]]
id = "backend"
label = "Backend"
command = ["dotnet", "watch"]
cwd = "backend"
auto_start = true
restart = "never"
port = 5000

[[workspace.processes]]
id = "frontend"
label = "Frontend"
command = ["npm", "run", "dev"]
cwd = "frontend"
auto_start = true
restart = "never"
port = 3000

[[workspace.agents]]
id = "codex"
command = ["codex"]
cwd = "."
auto_start = false

[[workspace.agents]]
id = "claude"
command = ["claude"]
cwd = "."
auto_start = false
```

## Decisión importante: comandos como arrays

Preferir:

```toml
command = ["npm", "run", "dev"]
```

sobre:

```toml
command = "npm run dev"
```

para evitar `shell=True`, problemas de quoting y ejecución accidental de operadores de shell.

Si en una versión futura se acepta un comando de shell, deberá ser explícito:

```toml
shell = true
command = "comando | otro-comando"
```

Y ChxChx deberá advertir que el comando usa shell.

---

# 8. Modelo de confianza y seguridad

Esta parte es obligatoria.

Si ChxChx empieza a leer comandos desde `.ai/chxchx-tech.toml`, **un repositorio clonado podría intentar ejecutar comandos maliciosos**.

Por eso un repositorio nuevo nunca debe ejecutar automáticamente procesos configurados hasta ser confiable.

## Flujo de confianza

```text
Repositorio nuevo
      │
      ▼
chxchx-tech init/open
      │
      ▼
lee configuración SIN ejecutarla
      │
      ▼
¿Proyecto confiable?
   │            │
  no            sí
   │            │
mostrar        ejecutar solo acciones
plan           permitidas
```

Estado local sugerido:

```text
~/.chxchx-tech-lead/
├── projects.json
├── trusted_projects.json
├── workspace-state.json
├── backups/
├── logs/
└── profiles/
```

Nunca guardar confianza dentro del propio repositorio, porque el repositorio podría modificarla.

## Reglas de ejecución

- No usar `shell=True` por defecto.
- No ejecutar `sudo` automáticamente.
- No ejecutar procesos de un proyecto no confiable.
- No imprimir valores completos de variables secretas.
- No copiar `.env` a logs.
- No almacenar tokens de Codex/Claude/OpenCode.
- No registrar el contenido de stdin de agentes.
- Mantener logs rotativos y acotados.
- Confirmar acciones destructivas.
- Mantener `--dry-run` para operaciones configurables.

---

# 9. Adaptadores

## 9.1 `TerminalWorkspaceAdapter`

Responsabilidad:

- crear sesión;
- detectar sesión;
- adjuntar sesión;
- cerrar sesión;
- abrir panes/tabs;
- ejecutar un proceso dentro de un pane;
- aplicar layout.

Primera implementación:

```text
ZellijAdapter
```

Fallback:

```text
SubprocessAdapter
```

El fallback permite usar ChxChx aunque Zellij no esté instalado, con menos funciones.

---

## 9.2 `EditorAdapter`

Interfaz mínima:

```text
available()
open_project(path)
open_file(path, line=None, column=None)
```

Primera implementación:

```text
SublimeAdapter
```

Futuras opcionales:

- VSCodeAdapter;
- NeovimAdapter;
- ZedAdapter.

No convertir ninguna en dependencia obligatoria.

---

## 9.3 `AgentAdapter`

Interfaz:

```text
available()
version()
start(project, session)
status()
```

Implementaciones:

- CodexAdapter;
- ClaudeAdapter;
- OpenCodeAdapter.

Los agentes deben ejecutarse con el `cwd` correcto y recibir contexto mediante los archivos existentes (`AGENTS.md`, `CLAUDE.md`, `.ai/`, MCP), no mediante prompts gigantes inyectados por ChxChx.

---

## 9.4 `DockerAdapter`

Primera versión:

- detectar `docker`;
- detectar `docker compose`;
- `up`;
- `stop`;
- `down` solo por acción explícita;
- listar servicios;
- estado básico;
- memoria aproximada mediante `docker stats --no-stream`.

`stop` debe ser preferido para cambios rápidos de proyecto. `down` será una acción separada porque destruye contenedores/redes efímeras.

---

## 9.5 Git

Primera versión solo lectura:

- branch;
- dirty/clean;
- cantidad de cambios;
- upstream/ahead/behind si puede obtenerse localmente.

No empezar construyendo un Git GUI.

Acciones de commit/push podrán añadirse después y nunca deben bloquear el MVP del workspace.

---

# 10. Process Manager

Debe ser independiente de la UI.

Responsabilidades:

- iniciar procesos;
- detener procesos;
- detectar PID;
- registrar hora de inicio;
- conocer cwd y comando;
- capturar estado de salida;
- identificar puertos cuando estén configurados;
- calcular RAM/CPU;
- evitar duplicar el mismo servicio accidentalmente.

Modelo sugerido:

```python
ManagedProcess(
    id: str,
    label: str,
    command: list[str],
    cwd: Path,
    status: ProcessStatus,
    pid: int | None,
    port: int | None,
    started_at: datetime | None,
)
```

Para métricas cross-platform, evaluar `psutil` como dependencia del workspace.

---

# 11. Resource Manager

Este módulo no es decorativo: responde directamente al motivo del proyecto.

Debe mostrar como mínimo:

- RAM total;
- RAM usada;
- RAM disponible;
- swap;
- CPU total;
- procesos gestionados por ChxChx;
- RAM/CPU de cada proceso gestionado;
- consumo Docker cuando sea posible.

Ejemplo:

```text
System RAM       8.4 / 16.0 GB   52%
Swap             0.3 / 8.0 GB     4%

Managed
frontend           720 MB
backend            590 MB
postgres           180 MB
codex              310 MB
claude             stopped
```

## Política inicial para máquinas de 16 GB

ChxChx no debe matar nada automáticamente.

Puede advertir:

```text
>= 75% RAM     WARNING
>= 90% RAM     CRITICAL
```

Y ofrecer acciones explícitas:

- detener proceso;
- detener Docker del proyecto;
- suspender workspace;
- cerrar agente.

---

# 12. Zellij como motor de workspace

Zellij será el primer motor de sesiones porque permite:

- sesiones nombradas;
- panes;
- layouts declarativos;
- reconexión;
- ejecución de comandos;
- control desde CLI.

ChxChx generará layouts temporales o administrados, no obligará al usuario a mantenerlos manualmente.

Ejemplo conceptual:

```text
Session: acore-crm

┌────────────────────────┬────────────────────────┐
│ CODEX                  │ CLAUDE                 │
├────────────────────────┴────────────────────────┤
│ BACKEND                                          │
├─────────────────────────────────────────────────┤
│ FRONTEND                                         │
└─────────────────────────────────────────────────┘
```

La implementación debe encapsular completamente los comandos Zellij en `adapters/terminal/zellij.py`.

---

# 13. TUI con Textual

Typer/Rich seguirán existiendo.

Textual será una capa adicional:

```text
Typer CLI ──────┐
                ├── WorkspaceService
Textual TUI ────┘
```

## Comportamiento esperado

```bash
chxchx-tech
```

abre la TUI cuando el terminal es interactivo.

Los comandos tradicionales siguen funcionando:

```bash
chxchx-tech doctor
chxchx-tech status
chxchx-tech init
chxchx-tech setup
```

Y se añaden comandos scriptables:

```bash
chxchx-tech workspace open
chxchx-tech workspace start
chxchx-tech workspace stop
chxchx-tech workspace attach
chxchx-tech process list
chxchx-tech process start frontend
chxchx-tech agent start codex
```

No obligar a usar la TUI para automatización.

---

# 14. Pantallas TUI

## Dashboard

Debe mostrar:

- proyecto;
- stack;
- Git;
- sesión;
- estado del workspace;
- procesos;
- agentes;
- Docker;
- RAM/CPU.

## Projects

```text
● ACore CRM          ACTIVE       2.2 GB
◐ OLAN Academic      SUSPENDED    120 MB
○ AlanIA MVP         STOPPED        0 MB
○ OTTO               STOPPED        0 MB
```

Acciones:

- abrir;
- activar;
- suspender;
- detener;
- abrir Sublime;
- iniciar agente.

## Processes

- status;
- PID;
- puerto;
- CPU;
- RAM;
- start/stop/restart;
- tail de logs controlado.

## Agents

- disponibilidad;
- versión;
- estado;
- iniciar en proyecto;
- adjuntar cuando el agente/terminal lo permita.

## Git

Primera versión informativa:

- branch;
- cambios;
- upstream.

## Command Palette

Aprovechar la Command Palette de Textual para acciones rápidas:

```text
Open project
Start frontend
Start Codex
Open Sublime
Stop Docker
Switch project
```

---

# 15. CLI objetivo

Los nombres exactos pueden ajustarse durante implementación, pero el contrato debe permanecer coherente.

```text
chxchx-tech
chxchx-tech tui

chxchx-tech workspace open [PATH|ALIAS]
chxchx-tech workspace start [PATH|ALIAS]
chxchx-tech workspace stop [PATH|ALIAS]
chxchx-tech workspace suspend [PATH|ALIAS]
chxchx-tech workspace resume [PATH|ALIAS]
chxchx-tech workspace attach [PATH|ALIAS]
chxchx-tech workspace status [PATH|ALIAS]

chxchx-tech process list
chxchx-tech process start ID
chxchx-tech process stop ID
chxchx-tech process restart ID

chxchx-tech agent list
chxchx-tech agent start codex
chxchx-tech agent start claude
chxchx-tech agent start opencode

chxchx-tech editor open

chxchx-tech resources

chxchx-tech projects list
chxchx-tech projects switch ALIAS
```

Los comandos actuales no deben romperse.

---

# 16. Flujo `chxchx-tech workspace open`

```text
1. resolver proyecto
2. detectar proyecto/stack
3. leer .ai/chxchx-tech.toml
4. validar schema
5. verificar confianza
6. comprobar dependencias
7. recuperar o crear WorkspaceState
8. comprobar sesión Zellij
9. crear/aplicar layout si es necesario
10. opcional: arrancar procesos marcados auto_start
11. opcional: abrir Sublime
12. mostrar estado
13. adjuntar sesión o volver a TUI según comando
```

No debe lanzar servicios si:

- el proyecto no es confiable;
- el config es inválido;
- falta cwd;
- el ejecutable no existe;
- el puerto configurado ya está ocupado y no corresponde al proceso esperado.

---

# 17. Flujo de cambio de proyecto

Objetivo:

```bash
chxchx-tech projects switch olan
```

Comportamiento:

```text
ACore CRM
   ACTIVE
     │
     ├── detener procesos auto-suspendibles
     ├── stop Docker opcional según política
     └── marcar SUSPENDED

OLAN Academic
   STOPPED/SUSPENDED
     │
     ├── recuperar sesión
     ├── iniciar procesos configurados
     ├── abrir/reenfocar editor
     └── marcar ACTIVE
```

## Regla

Solo un workspace será `ACTIVE` por defecto.

El usuario podrá tener más de uno activo explícitamente, pero ChxChx deberá advertir sobre RAM si supera el umbral.

---

# 18. Estrategia de fases

## Fase 0 — Congelar y proteger `v0.2.0`

### Objetivo

Tener una línea base verificable antes de tocar arquitectura.

### Tareas

- [x] Confirmar la etiqueta `v0.2.0` estable.
- [x] Ejecutar la suite actual (117 pruebas pasan en el entorno Linux disponible).
- [x] Añadir regresión CLI heredada para `init`, `setup`, `sync`, `doctor`, `rollback` y `projects`.
- [x] Confirmar mediante pruebas aisladas que `init`, `setup --dry-run`, `sync --dry-run`, `rollback` y `projects` mantienen sus comportamientos básicos.
- [x] ZIP no aplica a la distribución actual: se instala desde Git con `uv` ([guía de publicación](10-PUBLISH-GIT.md)); no hay artefacto ZIP de release que verificar.
- [x] Crear ADR: terminal-first sin reimplementar terminal/editor ([ADR-0001](adr/0001-terminal-first-workspace.md)).

### Definition of Done

El branch de desarrollo puede romper internamente sin perder una referencia estable y reproducible de `v0.2.0`.

> Validación local de cierre (2026-09-30): `compileall`, `pytest` (137 passed, 1 skipped), `uv lock --check` y `scripts/lab_smoke.py` completados correctamente en Linux.

> Fase 0 cerrada: la distribución documentada usa Git/`uv`; no publica ZIP.

---

## Fase 1 — Dominio del workspace y configuración v2

### Objetivo

Construir modelos y config sin ejecutar todavía terminales.

### Crear

```text
core/config_migrations.py
core/trust.py
workspace/models.py
workspace/state.py
workspace/manager.py
```

### Tareas

- [x] `WorkspaceConfig`.
- [x] `ProcessConfig`.
- [x] `AgentConfig`.
- [x] `WorkspaceState`.
- [x] enum de estados.
- [x] parser TOML v2.
- [x] migración v1 → v2 idempotente.
- [x] validación de IDs únicos.
- [x] validación de `cwd` relativo al repo.
- [x] sistema de confianza local.
- [x] `--dry-run` para migración.

### Definition of Done

Puede cargarse cualquier proyecto v1 existente y producirse un plan v2 sin ejecutar comandos externos.

---

## Fase 2 — Adaptadores externos

### Objetivo

Aislar herramientas de terceros.

### Implementar

- [x] `EditorAdapter` + Sublime.
- [x] `TerminalWorkspaceAdapter` + Zellij.
- [x] `AgentAdapter` + Codex/Claude/OpenCode.
- [x] Docker Compose adapter.
- [x] Git read-only adapter.
- [x] fallback `SubprocessAdapter`.

### Pruebas

Mockear el runner. Los tests no deben necesitar abrir Sublime, Zellij, Docker ni agentes reales.

### Definition of Done

Ningún servicio de dominio construye strings de comandos específicos de Zellij/Sublime/Docker.

---

## Fase 3 — MVP Workspace por CLI

### Objetivo

Hacer útil el sistema antes de construir la TUI.

### Comandos mínimos

```text
[x] chxchx-tech workspace status
[x] chxchx-tech workspace open
[x] chxchx-tech workspace start
[x] chxchx-tech workspace stop
[x] chxchx-tech workspace attach
[x] chxchx-tech editor open
[x] chxchx-tech process list
[x] chxchx-tech process start
[x] chxchx-tech process stop
[x] chxchx-tech agent start
```

### Definition of Done alcanzada para el MVP CLI

En un proyecto laboratorio se puede:

1. crear sesión;
2. iniciar backend/frontend;
3. abrir Sublime;
4. iniciar Codex o Claude;
5. detener los procesos;
6. volver a adjuntarse a la sesión;
7. repetir sin duplicar procesos.

Este es el primer milestone que debe usarse diariamente.

---

## Fase 4 — Textual TUI

### Objetivo

Agregar una interfaz agradable sin mover lógica fuera de `workspace/`.

### Alcance inicial

- [x] App shell.
- [x] Dashboard.
- [x] Projects screen.
- [x] Processes screen.
- [x] Agents screen.
- [x] navegación por teclado básica.
- [x] command palette.
- [x] refresco periódico del dashboard.

### No incluir todavía

- editor embebido;
- terminal embebida completa;
- diff Git complejo;
- chat IA dentro de Textual.

### Definition of Done inicial

El dashboard, las pantallas especializadas, el refresco de estado, la paleta de comandos y las acciones básicas de workspace/agentes/handoff están disponibles desde la TUI. El CLI continúa funcionando independientemente y la TUI no ejecuta subprocess directamente.

---

## Fase 5 — Resource Manager

### Objetivo

Controlar el consumo de la máquina.

### Implementar

- [x] RAM/CPU/swap global.
- [x] RAM/CPU de procesos gestionados.
- [x] Docker stats adapter básico.
- [x] warning configurable.
- [x] critical configurable.
- [x] vista `resources` por CLI y panel básico en TUI.
- [x] acciones manuales básicas para liberar recursos (`workspace stop`).

### Definition of Done

El usuario puede identificar desde ChxChx qué componentes del workspace están consumiendo RAM antes de abrir un monitor externo.

---

## Fase 6 — Multiproyecto real

### Objetivo

Pasar del registro actual a gestión operativa de proyectos.

### Implementar

- [x] alias de proyecto.
- [x] ACTIVE/SUSPENDED/STOPPED.
- [x] switch.
- [x] persistencia de último proyecto.
- [x] recuperación de sesiones.
- [x] política “un ACTIVE por defecto” al activar cualquier workspace.
- [x] vista agregada de recursos (`dev`, commit `c2b1867`).

### Definition of Done

Se puede cambiar entre tres repositorios sin mantener los tres stacks completos ejecutándose.

---

## Fase 7 — Agentes IA como parte del workspace

### Objetivo

Convertir Codex/Claude/OpenCode en herramientas del workspace, no ventanas aisladas.

### Implementar

- [x] detección/versiones.
- [x] iniciar agente en cwd correcto.
- [x] panel/status.
- [x] presets por proyecto.
- [x] reaprovechar MCP existente.
- [x] handoff explícito mediante `.ai/HANDOFF.md`.

### No hacer

No construir todavía un “supervisor autónomo” que deje múltiples agentes modificando el mismo árbol de trabajo sin control.

### Definition of Done

El usuario puede iniciar el agente correcto para el proyecto desde una sola interfaz y los agentes encuentran las reglas/contexto ya generados por ChxChx.

---

## Fase 8 — Hardening y cross-platform

### Matriz objetivo

| Plataforma | TUI | Sublime | Zellij | Workspace completo |
|---|---|---|---|---|
| Linux | Sí | Sí | Sí | Principal |
| macOS | Sí | Sí | Sí | Soportado |
| Windows + WSL | Sí | Windows/WSL bridge | Sí en WSL | Soportado |
| Windows nativo | Sí | Sí | adapter fallback | Parcial inicialmente |

### Tareas

- [x] Validar rutas relativas POSIX/Windows y bloquear escapes del proyecto.
- [x] Separar argumentos por defecto y permitir shell solo de forma explícita.
- [x] Detener grupos POSIX y árboles de procesos en Windows con verificación de propiedad; Windows usa `taskkill /T` antes de terminar el proceso padre y escala a `/F` si vence el tiempo de espera.
- [x] Guardar el estado global mediante escritura temporal y reemplazo atómico.
- [x] Rotar logs de procesos al superar 5 MiB y conservar hasta tres copias.
- [x] Migrar configuración v1 → v2 de forma validada e idempotente.
- [x] Recuperar el estado de procesos administrados desde disco tras reiniciar el manager.
- [x] Confirmar y documentar el PR verde en la matriz CI Windows, macOS y Linux (Python 3.11–3.13; 9 combinaciones, 2026-09-30).
- [x] Registrar la prueba manual reportada como exitosa en macOS y Windows 11 Pro nativo (2026-09-30).
- [ ] Antes de `v1.0`, anotar los escenarios manuales y revalidar en Windows 11 Pro la terminación de un proceso con hijos; decidir si WSL permanece en el alcance y validarlo si corresponde.
- [ ] Durante el uso real, registrar sesiones con la checklist de [17-REAL-WORLD-VALIDATION.md](17-REAL-WORLD-VALIDATION.md), validar comportamiento sostenido de shells/sesiones y recuperación tras cierres inesperados; validar WSL si permanece en el alcance.

---

## Fase 9 — `v1.0` Terminal Workspace

No declarar `v1.0` por tener muchas funciones.

Declararlo solo cuando:

- [ ] se usa diariamente en proyectos reales;
- [x] no detiene PIDs ajenos; cubierto por `tests/workspace/test_process_manager.py`.
- [x] recupera estado/procesos y reutiliza la sesión al reanudar; cubierto por `tests/workspace/test_process_manager.py` y `tests/workspace/test_service.py`.
- [x] las migraciones v1 → v2 son idempotentes y no ejecutan comandos; cubierto por `tests/workspace/test_config.py`.
- [x] el CLI tradicional mantiene sus flujos básicos; cubierto por `tests/test_cli.py`.
- [x] La TUI puede cerrarse sin detener el workspace; cubierto por `tests/tui/test_lifecycle.py`.
- [ ] el consumo propio de ChxChx es bajo;
- [ ] Linux/macOS y Windows 11 Pro están validados para la versión actual y documentados; validar WSL si se mantiene en el alcance;
- [x] existe documentación operativa de recuperación en [08-TROUBLESHOOTING.md](08-TROUBLESHOOTING.md).

---

# 19. Orden de implementación estricto

No empezar por “hacer una UI bonita”.

Orden:

```text
Config/Models
    ↓
Trust
    ↓
Adapters
    ↓
Workspace Manager
    ↓
Process Manager
    ↓
CLI MVP
    ↓
uso real
    ↓
Textual
    ↓
Resources
    ↓
Multiproyecto
    ↓
Agentes avanzados
```

Si el CLI MVP no funciona bien, la TUI solo maquillará problemas arquitectónicos.

---

# 20. Testing

## Unit tests

Crear:

```text
tests/workspace/
tests/adapters/
tests/tui/
```

Cubrir:

- config migration;
- trust;
- state transitions;
- command generation;
- process state;
- invalid cwd;
- duplicate IDs;
- unavailable executable;
- occupied ports;
- Zellij session parser;
- Git parser;
- Docker parser.

## Integration tests

Usar un proyecto laboratorio mínimo:

```text
tests/fixtures/workspace-demo/
```

Procesos de prueba deben ser inocuos, por ejemplo un pequeño servidor Python o proceso `sleep`, nunca depender de proyectos productivos.

## Smoke test

Extender `scripts/lab_smoke.py` para verificar:

```text
init
migrate config
trust lab
create workspace
start test process
status
stop test process
cleanup
```

---

# 21. Observabilidad

ChxChx necesita logs suficientes para depurar, pero no convertirse en un recolector de secretos.

Formato sugerido:

```text
~/.chxchx-tech-lead/logs/
├── chxchx-tech.log
└── workspaces/
    ├── acore-crm.log
    └── olan-academic.log
```

Guardar:

- timestamp;
- acción;
- proyecto;
- executable;
- PID;
- exit code;
- errores sanitizados.

No guardar:

- `.env`;
- contenido de prompts;
- stdin de agentes;
- tokens;
- Authorization headers;
- secretos detectables.

---

# 22. Gestión de RAM para la máquina objetivo

La herramienta debe diseñarse pensando también en equipos de 16 GB.

Política recomendada:

```text
1 proyecto ACTIVE
1-2 agentes según necesidad
Docker únicamente del proyecto activo
otros proyectos STOPPED o SUSPENDED
Sublime como editor principal
```

No se pretende impedir que el usuario active más proyectos. Se pretende que el coste sea visible.

Ejemplo:

```text
ACore CRM       ACTIVE        2.4 GB
OLAN Academic   SUSPENDED     0.2 GB
AlanIA MVP      STOPPED       0.0 GB
```

---

# 23. Gestión de Docker compartido

No automatizar desde la primera versión una infraestructura global compartida entre todos los repositorios.

Primero soportar correctamente Compose por proyecto.

Más adelante podrá estudiarse un modo opcional:

```text
shared-dev-infra
├── postgres
├── redis
└── rabbitmq
```

con múltiples DB/prefijos.

Eso debe ser una feature separada porque introduce acoplamiento entre proyectos.

---

# 24. Integración con Sublime

El adapter inicial solo necesita:

```text
subl <project-path>
subl <file>:<line>:<column>
```

ChxChx no necesita conocer plugins internos de Sublime.

Opcionalmente podrá generar un `.sublime-project` por workspace para excluir:

```text
node_modules
.next
dist
build
coverage
.git
.venv
```

Esto reduce indexación innecesaria.

---

# 25. UX de errores

Evitar mensajes genéricos como:

```text
Error starting workspace
```

Preferir:

```text
✗ No pude iniciar `frontend`
  Ejecutable: npm
  CWD: /home/user/Projects/acore/frontend
  Motivo: package.json no encontrado

  Acción sugerida:
  revisa `workspace.processes.frontend.cwd` en .ai/chxchx-tech.toml
```

La UI debe responder tres preguntas:

1. ¿qué falló?
2. ¿por qué?
3. ¿qué puedo hacer?

---

# 26. Compatibilidad con `v0.2.0`

Estos comandos deben conservarse:

```text
chxchx-tech version
chxchx-tech doctor
chxchx-tech install
chxchx-tech init
chxchx-tech setup
chxchx-tech status
chxchx-tech sync
chxchx-tech integrate
chxchx-tech rollback
chxchx-tech projects list
chxchx-tech projects sync
```

La evolución no debe convertir `chxchx-tech setup` en un comando que arranque servicios automáticamente.

Separar claramente:

```text
setup     → prepara/configura
workspace → ejecuta/opera
```

---

# 27. Migración de CLI para `chxchx-tech`

Actualmente el CLI usa `no_args_is_help=True`.

Para que `chxchx-tech` abra la TUI habrá que modificar el entrypoint conservando:

```bash
chxchx-tech --help
```

Una estrategia válida es un callback con ejecución sin subcomando:

```text
si hay subcomando → Typer normal
si no hay subcomando + TTY → TUI
si no hay TTY → mostrar help/error seguro
```

Nunca lanzar TUI en pipelines/CI.

---

# 28. Dependencias nuevas

Mantener pocas.

Base esperada:

```text
typer
rich
textual
psutil
```

`textual-dev` debe ser dependencia de desarrollo, no runtime.

Zellij, Sublime, Docker y agentes son herramientas externas detectadas por `doctor`, no paquetes Python instalados dentro del entorno de ChxChx.

---

# 29. Doctor v0.3

Debe separar:

```text
CORE
✓ Python
✓ uv
✓ Git

WORKSPACE
✓ Zellij
✓ Sublime
✓ Docker

AGENTS
✓ Codex
✓ Claude
✗ OpenCode

PROJECT
✓ .ai/chxchx-tech.toml v2
✓ trusted
✓ backend cwd
✓ frontend cwd
! port 3000 ocupado
```

Un componente opcional faltante no debe marcar toda la instalación como rota.

---

# 30. Backups y rollback

El mecanismo existente de backups debe ampliarse a:

- migración de `chxchx-tech.toml`;
- configuración administrada de layouts, si se persiste;
- cualquier archivo generado dentro del proyecto.

No intentar rollback de procesos del SO mediante el sistema de backups de archivos.

Los procesos se recuperan mediante WorkspaceState, no mediante copias de archivos.

---

# 31. Release ZIP / distribución

No empaquetar:

```text
.git/
.venv/
__pycache__/
.pytest_cache/
*.pyc
logs/
```

Distribuir únicamente código/fuentes/documentación necesaria.

La instalación debe recrear dependencias mediante `uv`.

---

# 32. Estado de milestones de release

El siguiente mapa refleja implementación en la rama `dev`, no publicación de esas versiones:

- `v0.3.0` — Terminal Workspace MVP: implementado.
- `v0.3.1` — TUI inicial: implementado.
- `v0.3.2` — Resource Manager: implementado.
- `v0.4.0` — Operación multiproyecto: implementada.
- `v0.5.0` — Hardening para uso diario: en curso; CI verde en los tres sistemas y pruebas manuales exitosas reportadas en macOS y Windows 11 Pro. Pendientes: evidencia de uso sostenido y validación manual de release.
- `v1.0.0` — solo después de cumplir los criterios de la fase 9 y publicar los resultados de validación.

---

# 33. Definition of Success

El proyecto habrá alcanzado el objetivo cuando el flujo normal de desarrollo sea:

```bash
cd ~/Projects/acore-crm
chxchx-tech
```

Y desde ahí pueda:

- ver el estado del proyecto;
- abrir Sublime;
- iniciar backend/frontend;
- levantar Docker;
- iniciar Codex/Claude;
- ver consumo de RAM;
- detener componentes;
- cambiar de proyecto;
- volver a una sesión existente;

sin necesitar varias ventanas pesadas de IDE y sin perder la capacidad de operar todo mediante CLI.

---

# 34. Regla contra scope creep

Antes de añadir una feature nueva, responder:

```text
¿Esto ayuda a iniciar, operar, entender o cambiar de workspace?
```

Si la respuesta es no, no pertenece al core de ChxChx Terminal Workspace.

Ejemplos que deben esperar:

- navegador web embebido;
- IDE completo;
- gestor de tickets;
- cliente de correo;
- sistema autónomo de agentes sin supervisión;
- reemplazo de GitHub/GitLab;
- editor visual de Docker Compose.

---

# 35. Primera ejecución recomendada de implementación

La primera PR de esta evolución debe ser pequeña:

```text
feat(workspace): introduce workspace domain and config v2
```

Debe contener únicamente:

- modelos;
- parser;
- migración;
- trust model;
- tests;
- documentación.

No Textual. No Zellij real. No ejecución de procesos.

La segunda PR:

```text
feat(workspace): add terminal and editor adapters
```

La tercera:

```text
feat(workspace): add CLI workspace MVP
```

Solo después:

```text
feat(tui): add terminal workspace dashboard
```

Así el proyecto evoluciona sin convertir una base estable en una reescritura imposible de depurar.

---

# 36. Resultado conceptual final

```text
                         CHXCHX TECH LEAD
                                │
                  ┌─────────────┴─────────────┐
                  │                           │
                Typer                      Textual
                  │                           │
                  └─────────────┬─────────────┘
                                │
                        Workspace Manager
                                │
          ┌─────────────┬───────┼────────┬──────────────┐
          │             │       │        │              │
       Processes      Editor   Git     Docker         Agents
          │             │       │        │              │
       Zellij         Sublime   CLI    Compose   Codex/Claude/OpenCode
          │
      Sessions
          │
  backend/frontend/logs
```

ChxChx no reemplaza las herramientas. **Las convierte en un único flujo de trabajo ligero, reproducible y controlado.**
