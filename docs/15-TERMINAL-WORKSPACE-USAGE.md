# 15 — Instalación y uso: ChxChx Terminal Workspace

> Guía operativa para el flujo terminal-first de `chxchx-tech-lead`.
>
> **Estado:** `v0.2.0` es la última base publicada. Terminal Workspace está implementado en la rama `dev`; esta guía describe su uso actual y las funciones que deben validarse en el rollout multiplataforma antes de `v1.0`.

---

# 1. Filosofía de uso

El entorno recomendado será:

```text
Sublime Text      → edición/inspección de código
Terminal          → interfaz principal
Zellij            → sesiones, panes y persistencia
ChxChx           → orquestación
Codex/Claude      → agentes IA
Docker            → infraestructura solo cuando se necesita
Git CLI           → control de versiones
```

El objetivo es evitar que un IDE pesado sea el centro de todo el entorno.

Flujo esperado:

```text
abrir terminal
     ↓
cd proyecto
     ↓
chxchx-tech
     ↓
seleccionar/iniciar workspace
     ↓
Sublime + procesos + agentes
```

---

# 2. Requisitos base

## Obligatorios

- Git.
- Python `>=3.11` para la versión actual del proyecto.
- `uv`.
- una terminal moderna.

## Recomendados para Terminal Workspace

- Zellij.
- Sublime Text.
- Docker + Compose cuando el proyecto lo necesite.
- Codex CLI, Claude Code u OpenCode según el flujo personal.

## Opcionales

- `btop` como monitor externo de respaldo.
- Kitty, WezTerm, Ghostty u otro terminal ligero.

ChxChx no debe instalar automáticamente herramientas de escritorio ni clientes IA que tengan su propio ciclo de instalación. `doctor` debe detectarlos y explicar qué falta.

Los scripts `scripts/install.sh` y `scripts/install.ps1` sí instalan
automáticamente `uv`, ChxChx y las herramientas gestionadas por ChxChx
(Basic Memory y Serena). Zellij, Sublime, Docker y los clientes IA se detectan
con `doctor` y se instalan siguiendo sus instrucciones oficiales.

---

# 3. Instalar `uv`

Linux/macOS:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Después abre una nueva terminal o agrega `~/.local/bin` al `PATH` si fuera necesario.

Comprobar:

```bash
uv --version
```

En Windows PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
```

---

# 4. Instalar ChxChx `v0.2.0` actual

Desde Git:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
```

Verificar:

```bash
chxchx-tech version
chxchx-tech doctor
```

Resultado esperado de versión:

```text
chxchx-tech-lead 0.2.0
```

---

# 5. Instalar el proyecto en modo desarrollo

Para trabajar en el propio `chxchx-tech-lead`:

```bash
git clone <URL-DE-TU-REPOSITORIO>
cd chxchx-tech-lead
uv sync --dev
uv tool install --editable .
```

Verificar:

```bash
chxchx-tech version
uv run pytest -q
```

Textual ya está incluido como dependencia runtime porque `chxchx-tech tui` es una función disponible. Las herramientas externas (Zellij, Sublime, Docker y agentes) siguen siendo opcionales y se detectan, no se instalan desde el paquete Python.

---

# 6. No distribuir entornos locales

No agregar a releases ni ZIPs:

```text
.git/
.venv/
__pycache__/
.pytest_cache/
*.pyc
```

El receptor debe reconstruir el entorno con:

```bash
uv sync
```

o instalar el CLI con:

```bash
uv tool install ...
```

---

# 7. Instalar Zellij

Zellij será el motor recomendado para sesiones y panes.

Comprueba primero:

```bash
zellij --version
```

Si tu distribución ofrece un paquete mantenido, puedes usar su gestor de paquetes. Si no, utiliza una de las rutas publicadas por Zellij:

- binario precompilado de la release;
- `cargo binstall zellij`;
- `cargo install --locked zellij`.

En portátiles conviene preferir un binario precompilado antes que compilar Zellij únicamente para instalarlo.

Documentación oficial:

- https://zellij.dev/documentation/installation

## Probar Zellij

```bash
zellij
```

Salir de la sesión no implica necesariamente destruirla; el objetivo de ChxChx será aprovechar precisamente esa persistencia.

Listar sesiones:

```bash
zellij list-sessions
```

Adjuntar:

```bash
zellij attach NOMBRE
```

---

# 8. Instalar Sublime Text

Instala Sublime Text 4 desde el repositorio/paquete oficial correspondiente a tu sistema.

Verifica que el comando esté disponible:

```bash
subl --version
```

O prueba:

```bash
subl .
```

Si Sublime abre el directorio actual, el adapter podrá utilizarlo.

Sitio oficial:

- https://www.sublimetext.com/download

## Plugins recomendados

Mantener Sublime ligero.

Base sugerida:

```text
Package Control
LSP
LSP-typescript          solo si realmente lo necesitas
EditorConfig
GitSavvy                opcional
```

No recrear VS Code instalando decenas de paquetes.

---

# 9. Verificar agentes IA

ChxChx no debe asumir que todos existen.

Comprueba los que uses:

```bash
codex --version
claude --version
opencode --version
```

No es obligatorio instalar los tres.

El flujo recomendado en una máquina de 16 GB es iniciar únicamente el agente que estés usando.

---

# 10. Flujo actual `v0.2.0`

Desde la raíz de un proyecto:

```bash
chxchx-tech doctor
chxchx-tech install
chxchx-tech init --dry-run
chxchx-tech init
chxchx-tech status
```

O flujo completo:

```bash
chxchx-tech setup --dry-run
chxchx-tech setup
```

Esto prepara:

```text
AGENTS.md
CLAUDE.md
.ai/
.ai/chxchx-tech.toml
.ai/memory/
docs/adr/
```

En la rama de desarrollo del Terminal Workspace, `.ai/chxchx-tech.toml` se genera en
`version = 2` y las configuraciones existentes `version = 1` se migran durante
`chxchx-tech init`. Para una configuración nueva, el análisis sugiere un proceso
inicial cuando reconoce un comando convencional (por ejemplo, React Native usa
el script `start` con el gestor detectado, como `pnpm start`; también reconoce
Vite, Next.js, NestJS, Django, .NET, Rust y Go). Las sugerencias quedan visibles
en la TUI y se inician al iniciar el workspace, después de confiar el repositorio.
La inicialización nunca ejecuta esos comandos por sí sola.

También registra el repositorio en:

```text
~/.chxchx-tech-lead/projects.json
```

---

# 11. Integrar MCP en `v0.2.0`

Ejemplo Claude:

```bash
chxchx-tech integrate --dry-run --client claude
chxchx-tech integrate --client claude
```

Codex:

```bash
chxchx-tech integrate --dry-run --client codex
chxchx-tech integrate --client codex
```

OpenCode:

```bash
chxchx-tech integrate --dry-run --client opencode
chxchx-tech integrate --client opencode
```

ChxChx no debe guardar tokens de estos clientes.

---

# 12. Uso actual: primer uso de Terminal Workspace

Con Terminal Workspace instalado desde la rama de desarrollo, el flujo es:

```bash
cd ~/Projects/mi-proyecto
chxchx-tech workspace open
```

O, cuando `chxchx-tech` sin argumentos lance la TUI:

```bash
chxchx-tech
```

Primera apertura:

```text
Proyecto: mi-proyecto
Config: .ai/chxchx-tech.toml
Trust: no configurado

Este repositorio contiene comandos que ChxChx puede ejecutar.
Revisa el plan antes de confiar en él.
```

El proyecto no debe arrancar comandos automáticamente hasta haber sido marcado como confiable de forma local.

---

# 13. Uso actual: configurar el workspace

Ejemplo Full Stack:

```toml
version = 2
profile = "node"
memory_project = "mi-proyecto-a1b2c3"
memory_path = ".ai/memory"

[workspace]
name = "mi-proyecto"
adapter = "zellij"
editor = "sublime"
auto_open_editor = false
auto_attach = true
auto_start = false

[workspace.resources]
warn_memory_percent = 75
critical_memory_percent = 90

[workspace.docker]
enabled = true
compose_file = "compose.yaml"
auto_start = false

[[workspace.processes]]
id = "backend"
label = "Backend"
command = ["npm", "run", "dev"]
cwd = "backend"
auto_start = true
port = 3001

[[workspace.processes]]
id = "frontend"
label = "Frontend"
command = ["npm", "run", "dev"]
cwd = "frontend"
auto_start = true
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

---

# 14. Ejemplo `.NET + Next.js`

```toml
[[workspace.processes]]
id = "api"
label = "API .NET"
command = ["dotnet", "watch"]
cwd = "backend"
auto_start = true
port = 5000

[[workspace.processes]]
id = "web"
label = "Next.js"
command = ["npm", "run", "dev"]
cwd = "frontend"
auto_start = true
port = 3000
```

No poner:

```toml
command = ["cd", "backend", "&&", "dotnet", "watch"]
```

Usar `cwd` para cambiar de directorio.

---

# 15. Ejemplo NestJS + React

```toml
[[workspace.processes]]
id = "api"
label = "NestJS"
command = ["npm", "run", "start:dev"]
cwd = "apps/api"
auto_start = true
port = 3001

[[workspace.processes]]
id = "web"
label = "React"
command = ["npm", "run", "dev"]
cwd = "apps/web"
auto_start = true
port = 5173
```

---

# 16. Ejemplo React Native

Metro puede gestionarse desde ChxChx:

```toml
[[workspace.processes]]
id = "metro"
label = "Metro"
command = ["npm", "start"]
cwd = "."
auto_start = true
port = 8081
```

No intentar meter Android Studio/Xcode completos dentro de la TUI. ChxChx puede ofrecer un adapter/launcher futuro, pero esas herramientas conservan su propio ciclo de vida.

---

# 17. Uso actual: flujo diario

## Iniciar

```bash
cd ~/Projects/acore-crm
chxchx-tech
```

Desde la TUI:

```text
Start workspace
Open Sublime
Start backend
Start frontend
Start Codex
```

O por CLI:

```bash
chxchx-tech workspace start
chxchx-tech editor open
chxchx-tech process start backend
chxchx-tech process start frontend
chxchx-tech agent start codex
```

---

# 18. Uso actual: trabajar con sesiones

Adjuntar al workspace actual:

```bash
chxchx-tech workspace attach
```

El adapter podrá traducirlo internamente a la sesión Zellij correspondiente.

La sesión debe tener un nombre estable derivado del proyecto, por ejemplo:

```text
acore-crm
olan-academic
alania-mvp
```

No usar nombres generados aleatoriamente en cada inicio porque impediría recuperación sencilla.

---

# 19. Uso actual: detener sin destruir

Para dejar de consumir RAM pero mantener configuración:

```bash
chxchx-tech workspace stop
```

Esto debe detener procesos gestionados, pero no borrar configuración ni archivos del proyecto.

Docker debe preferir:

```text
stop
```

sobre:

```text
down
```

para un cambio temporal de proyecto.

`down` será una acción explícita.

---

# 20. Suspender y cambiar de proyecto

```bash
chxchx-tech workspace suspend
```

Estado esperado:

```text
ACore CRM       SUSPENDED
```

Después:

```bash
chxchx-tech projects switch olan
```

Resultado:

```text
ACore CRM       SUSPENDED
OLAN Academic   ACTIVE
```

`projects switch` suspende el workspace activo, detiene sus procesos gestionados y activa el
destino. El alias se genera automáticamente al registrar cada proyecto.

---

# 21. Política recomendada para 16 GB de RAM

Mantener:

```text
1 workspace ACTIVE
0-1 workspace SUSPENDED
resto STOPPED
```

Agentes:

```text
Codex o Claude activo según tarea
no mantener ambos por costumbre
```

Docker:

```text
solo servicios necesarios del proyecto activo
```

Editor:

```text
una instancia de Sublime con uno o varios proyectos ligeros
```

Navegador:

Mantener bajo control pestañas de documentación, DevTools y aplicaciones pesadas.

---

# 22. Uso actual: ver consumo

```bash
chxchx-tech resources
```

Salida conceptual:

```text
System
RAM       8.4 / 16.0 GB     52%
Swap      0.3 / 8.0 GB       4%
CPU                         24%

Managed processes
frontend       720 MB
backend        590 MB
postgres       180 MB
codex          310 MB
```

ChxChx solo debe sugerir acciones. No debe matar procesos automáticamente por cruzar un umbral.

---

# 23. Uso actual: abrir Sublime

Desde cualquier proyecto configurado:

```bash
chxchx-tech editor open
```

Internamente, el adapter de Sublime podrá ejecutar:

```bash
subl /ruta/al/proyecto
```

Para un archivo concreto, una futura acción puede resolver:

```bash
subl archivo.py:120:8
```

---

# 24. Sublime: exclusiones recomendadas

Si se genera un `.sublime-project`, excluir carpetas de build/dependencias:

```json
{
  "folders": [
    {
      "path": ".",
      "folder_exclude_patterns": [
        ".git",
        ".next",
        ".venv",
        "build",
        "coverage",
        "dist",
        "node_modules"
      ]
    }
  ]
}
```

No es obligatorio generar este archivo para el MVP.

---

# 25. Uso actual: iniciar agentes

Codex:

```bash
chxchx-tech agent start codex
```

Claude:

```bash
chxchx-tech agent start claude
```

OpenCode:

```bash
chxchx-tech agent start opencode
```

El agente debe iniciarse con el root del proyecto como `cwd` salvo configuración distinta.

Para revisar agentes sin iniciarlos:

```bash
chxchx-tech agent list --path .
```

Los presets se declaran en `[workspace.presets]` como listas de IDs configurados en
`[[workspace.agents]]`. El handoff se actualiza con `workspace handoff` y conserva contenido
manual fuera del bloque administrado.

Antes de inventar un prompt enorme, se apoyará en:

```text
AGENTS.md
CLAUDE.md
.ai/
docs/
Basic Memory
Serena
```

---

# 26. TUI: navegación y teclas

La TUI operativa se inicia con `chxchx-tech tui [PATH]` y conserva el workspace
al cerrarse. El layout usa pestañas para evitar mezclar el estado de proyectos,
agentes y procesos:

```text
1       Resumen
2       Proyectos
3       Agentes
4       Procesos
5       Recursos
6       Handoff
7       Notas del proyecto
8       Chats
9       Marca / sello CHXCHX-DEV
Ctrl+P  Paleta de comandos
r       Actualizar
o       Abrir/preparar workspace
j       Adjuntar a Zellij
y       Marcar proyecto como confiable
s       Iniciar workspace
t       Iniciar workspace y agentes
a       Iniciar agentes
c       Iniciar agente indicado
g       Abrir terminal del agente seleccionado
i       Iniciar proceso indicado
k       Detener proceso indicado
u       Suspender workspace
v       Reanudar workspace
x       Detener workspace
h       Actualizar handoff
e       Abrir Sublime
q       Cerrar TUI
```

`q` cierra la interfaz, **no mata el workspace**.

Al usar `j` o `Adjuntar Zellij`, la TUI muestra la instrucción de retorno. Desde
Zellij pulsa `Ctrl+O`, suelta las teclas y después pulsa `D`; eso te devuelve a
la TUI sin detener el workspace. La TUI suspende y restaura su control del
terminal alrededor del adjunto para que el teclado siga funcionando al volver.
Antes de adjuntar se comprueba que la sesión
exista y esté activa. Si Zellij no la reconoce, la operación termina con un
error accionable y recomienda revisar `zellij list-sessions` y volver a iniciar
el workspace.

Detener el workspace debe ser una acción distinta y explícita.

---

# 27. TUI: Command Palette

La paleta local se abre con `Ctrl+P`, permite filtrar por texto y ejecuta las acciones del mismo `WorkspaceService` que usa la CLI.

La pestaña **Chats** muestra, en modo lectura, las sesiones locales de Codex y
Claude Code asociadas al proyecto seleccionado. Permite buscar por título o
último mensaje y leer el transcript de cada sesión. La pestaña **Notas** conserva
el historial persistente de `.ai/memory`. En **Agentes**, selecciona Codex o
Claude y pulsa **Abrir terminal del agente seleccionado** (o `g`) para entrar
a su pane interactivo de Zellij; al salir, vuelve a la TUI.

El Resumen muestra el flujo guiado `Confiar → Iniciar workspace + agentes →
Reintentar agentes → Adjuntar Zellij`; esas acciones no requieren introducir texto. La pestaña
Procesos incluye controles para iniciar y detener un proceso por su
ID. Recursos permite actualizar las métricas o detener el workspace. Handoff
permite editar resumen, pendientes y validación antes de actualizar el bloque
administrado de `.ai/HANDOFF.md`.

Ejemplos:

```text
Ver proyectos
Iniciar workspace
Iniciar agentes
Actualizar handoff
Cambiar a un proyecto desde la pestaña Proyectos
```

Esto evita crear docenas de atajos difíciles de recordar.

---

# 28. Proyectos registrados

La operación multiproyecto permite:

```bash
chxchx-tech projects list
chxchx-tech projects current
chxchx-tech projects switch olan --no-attach
chxchx-tech projects sync
```

El registro muestra el estado operativo persistido:

```text
NAME             STATE        RAM
ACore CRM        ACTIVE       2.4 GB
OLAN Academic    SUSPENDED    0.2 GB
AlanIA MVP       STOPPED      0.0 GB
```

---

# 29. Recuperar después de cerrar la terminal

Si Zellij sigue con la sesión viva:

```bash
chxchx-tech workspace attach
```

O directamente:

```bash
zellij list-sessions
zellij attach NOMBRE
```

ChxChx debe detectar si la sesión registrada ya no existe y corregir su estado local en vez de quedar permanentemente en `ACTIVE` fantasma.

---

# 30. Si un puerto está ocupado

Ejemplo:

```text
frontend → :3000
```

ChxChx debe comprobar el puerto antes de lanzar otra copia.

El mensaje esperado debe ser similar a:

```text
! No se inició frontend.
  El puerto 3000 ya está ocupado.

  PID detectado: 12345
  El proceso no pertenece al workspace actual.
```

No matar ese PID automáticamente.

---

# 31. Si Zellij no está instalado

El sistema debe poder degradarse:

```text
Workspace adapter: subprocess
```

Funciones disponibles:

- iniciar/detener procesos;
- recursos;
- editor;
- agentes básicos.

Funciones reducidas:

- layouts;
- pane management;
- attach persistente.

`doctor` debe recomendar Zellij sin considerar todo ChxChx inutilizable.

---

# 32. Si Sublime no está instalado

El workspace seguirá siendo funcional.

```text
Editor: unavailable
```

`chxchx-tech editor open` deberá mostrar una instrucción clara y no fallar el backend/frontend.

---

# 33. Si Docker no está instalado

Solo los proyectos que hayan configurado Docker deben mostrar advertencia.

Un proyecto Python/Node sin Docker no debe ser marcado como roto.

---

# 34. `doctor` para Terminal Workspace

```bash
chxchx-tech doctor
```

Ejemplo:

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
! OpenCode unavailable

PROJECT
✓ .ai/chxchx-tech.toml v2
✓ trusted
✓ frontend cwd
✓ backend cwd
✓ ports available
```

---

# 35. Seguridad: repositorios clonados

Nunca hagas:

```bash
git clone repositorio-desconocido
cd repositorio-desconocido
chxchx-tech workspace start --yes-to-everything
```

El diseño debe obligar a revisar el plan de ejecución la primera vez.

La confianza se guarda localmente fuera del repositorio.

Si `.ai/chxchx-tech.toml` cambia de forma relevante, ChxChx puede volver a pedir revisión según la estrategia de seguridad implementada.

---

# 36. Seguridad: variables de entorno

`.env` pertenece al proyecto y a su sistema de secretos.

ChxChx:

- puede pasar el entorno heredado al proceso cuando corresponda;
- puede permitir configurar nombres de archivos env;
- no debe copiar su contenido a logs;
- no debe guardar secretos en `projects.json`;
- no debe convertir `chxchx-tech.toml` en un almacén de credenciales.

---

# 37. Logs

Los logs de procesos se guardan en:

```text
~/.chxchx-tech-lead/logs/workspaces/
```

Cada archivo rota al alcanzar 5 MiB y conserva hasta tres copias anteriores. Los logs pueden contener la salida de los procesos configurados; no se deben imprimir secretos desde esos procesos.

Para diagnosticar:

```bash
chxchx-tech doctor
chxchx-tech workspace status
chxchx-tech process list
```

Todavía no existe un subcomando dedicado para consultar logs.

---

# 38. Rollback de configuración

La función actual se mantiene:

```bash
chxchx-tech rollback
```

Úsala para revertir archivos administrados por ChxChx.

No debe confundirse con detener procesos.

```text
rollback          → archivos/configuración
workspace stop    → procesos
```

---

# 39. Desarrollo del propio ChxChx

Entrar al repositorio:

```bash
cd chxchx-tech-lead
uv sync --dev
```

Tests:

```bash
uv run pytest -q
```

Compilación Python:

```bash
uv run python -m compileall -q src tests scripts
```

Smoke actual:

```bash
uv run python scripts/lab_smoke.py
```

Antes de cada release futura deberán mantenerse estas verificaciones y extender el smoke al workspace.

---

# 40. Cierre de Terminal Workspace

Las fases de dominio, adapters, CLI, TUI, recursos, multiproyecto y agentes ya están implementadas en `dev`. El trabajo pendiente está registrado en [05-ROADMAP.md](05-ROADMAP.md): ejecutar validación nativa en Linux/macOS/Windows + WSL y recopilar evidencia de uso diario para `v1.0`.

Antes de preparar una release, ejecuta desde la raíz:

```bash
uv run python -m compileall -q src tests scripts
uv run pytest -q
uv run python scripts/lab_smoke.py
uv lock --check
```

El plan de fases y sus criterios de aceptación permanecen en [14-TERMINAL-WORKSPACE-PLAN.md](14-TERMINAL-WORKSPACE-PLAN.md).

---

# 41. Actualización futura desde `v0.2.0`

Antes de migrar un proyecto real:

```bash
chxchx-tech status
chxchx-tech init --dry-run
```

La migración a config v2 deberá ser visible mediante dry-run.

Después:

```bash
chxchx-tech init
```

La operación debe generar backup antes de modificar `.ai/chxchx-tech.toml`.

---

# 42. Desinstalar una instalación `uv tool`

```bash
uv tool uninstall chxchx-tech-lead
```

Esto no debe borrar automáticamente:

```text
.ai/
AGENTS.md
CLAUDE.md
~/.chxchx-tech-lead/backups/
```

La eliminación de estado local deberá ser una acción separada y explícita si se implementa.

---

# 43. Comandos rápidos — versión actual

```bash
# comprobar
chxchx-tech version
chxchx-tech doctor

# preparar proyecto
chxchx-tech init --dry-run
chxchx-tech init

# todo el setup
chxchx-tech setup --dry-run
chxchx-tech setup

# estado
chxchx-tech status

# MCP
chxchx-tech integrate --client codex
chxchx-tech integrate --client claude

# proyectos
chxchx-tech projects list
chxchx-tech projects current
chxchx-tech projects switch acore
chxchx-tech projects sync

# recuperación
chxchx-tech rollback
```

---

# 44. Comandos rápidos — objetivo Terminal Workspace

En la implementación actual ya están disponibles `workspace status`,
`workspace trust`, `process list/start/stop` y los adapters internos para
agentes, Docker Compose y Git informativo. También están disponibles
`workspace open/start/stop/suspend/resume/attach`, `editor open`, `agent start`,
`agent list`, `agent status` y la operación multiproyecto mediante `projects switch`.

El comando `chxchx-tech resources` ya muestra RAM, swap, CPU y métricas de procesos
administrados. Los umbrales solo generan advertencias: ChxChx no mata procesos
automáticamente.

La TUI se inicia con `chxchx-tech tui [PATH]` y requiere la dependencia runtime
`textual`. Incluye pestañas de resumen, proyectos, agentes, procesos, recursos,
handoff, notas, chats y marca. El análisis también propone procesos de desarrollo
para stacks reconocidos en configuraciones nuevas; solo se ejecutan al iniciar
manualmente un proyecto confiable. Si Textual no está instalado, el comando
informa cómo completar el entorno sin afectar el resto del CLI.

```bash
# interfaz
chxchx-tech

# workspace
chxchx-tech workspace open
chxchx-tech workspace start
chxchx-tech workspace attach
chxchx-tech workspace status
chxchx-tech workspace trust
chxchx-tech workspace suspend
chxchx-tech workspace stop

# procesos
chxchx-tech process list
chxchx-tech process start frontend
chxchx-tech process stop frontend

# editor
chxchx-tech editor open

# agentes
chxchx-tech agent start codex
chxchx-tech agent start claude

# recursos
chxchx-tech resources

# multiproyecto
chxchx-tech projects switch acore
chxchx-tech projects switch olan
```

---

# 45. Flujo recomendado para el portátil de 16 GB

Ejemplo de sesión de trabajo:

```bash
cd ~/Projects/acore-crm
chxchx-tech
```

Estado:

```text
ACore CRM       ACTIVE
OLAN            STOPPED
AlanIA          STOPPED
```

Abrir:

```text
Sublime
backend
frontend
Docker necesario
Codex
```

Si necesitas OLAN:

```text
suspender ACore
activar OLAN
```

No mantener por costumbre:

```text
ACore frontend + backend
OLAN frontend + backend
AlanIA frontend + backend
2-3 Docker Compose
Codex
Claude
VS Code x3
```

El propósito de ChxChx Terminal Workspace es precisamente que esta disciplina deje de depender de memoria manual.

---

# 46. Resultado esperado

Cuando la evolución esté terminada, tu flujo debe reducirse a:

```bash
chxchx-tech
```

Y desde una sola interfaz poder decidir:

```text
qué proyecto está activo
qué proceso corre
qué agente usar
qué abrir en Sublime
cuánta RAM consume
qué detener
qué proyecto activar después
```

Sin reimplementar las herramientas que ya resuelven bien terminal, edición, Git, Docker o IA.

---

# 47. Referencias técnicas

- Textual: https://textual.textualize.io/
- Textual Getting Started: https://textual.textualize.io/getting_started/
- Zellij: https://zellij.dev/
- Zellij Installation: https://zellij.dev/documentation/installation
- Zellij Commands: https://zellij.dev/documentation/commands
- Zellij Layouts: https://zellij.dev/documentation/layouts.html
- Sublime Text: https://www.sublimetext.com/download
- uv: https://docs.astral.sh/uv/
