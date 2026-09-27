# chxchx-tech-lead

Bootstrapper y control plane local para repositorios de desarrollo asistido por agentes.

ChxChx prepara el contexto de un proyecto, conecta herramientas externas y administra un workspace ligero desde la terminal. No reemplaza a Git, Docker, Zellij, Sublime, Codex, Claude ni OpenCode: los detecta y los orquesta mediante adapters.

## Funciones

La versión pública `0.2.0` incluye:

- configuración `.ai/chxchx-tech.toml` v2 y migración v1 → v2;
- trust local para impedir ejecutar comandos de repositorios no aprobados;
- Process Manager con estado persistente, logs y protección contra PIDs ajenos;
- adapters para Zellij, fallback de subprocess, Sublime, agentes CLI, Docker Compose y Git;
- comandos CLI de workspace, procesos, agentes, editor y recursos;
- dashboard inicial con Textual;
- instalación reproducible con `uv` y una TUI operativa.
- operación multiproyecto mediante proyectos registrados y sesiones recuperables.

## Requisitos

Obligatorios:

- Python `>=3.11`;
- Git;
- `uv`;
- una terminal.

Para usar todas las funciones del workspace, instala también según necesidad:

- Zellij para sesiones persistentes;
- Sublime Text para editar;
- Docker y Compose para infraestructura;
- Codex CLI, Claude Code u OpenCode para agentes.

ChxChx instala únicamente las herramientas gestionadas por él: Basic Memory y Serena. Los clientes de escritorio, Docker, Zellij y agentes mantienen su instalación oficial separada.

### Instalador reducido

En Linux/macOS puedes instalarlo sin clonar el repositorio:

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/main/scripts/bootstrap.sh | sh
```

En Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/main/scripts/bootstrap.ps1 | iex
```

Estos instaladores solo dejan `chxchx-tech` como herramienta global; la configuración de cada proyecto se crea aparte.

## Instalación desde un clon

### Linux y macOS

Desde la raíz de este repositorio:

```bash
sh scripts/install.sh
```

El script:

1. instala `uv` si no existe;
2. instala ChxChx como herramienta editable mediante `uv`;
3. instala Basic Memory y Serena mediante `uv tool`;
4. deja la comprobación final en `chxchx-tech doctor`.

Si el comando todavía no aparece, abre una terminal nueva o ejecuta `uv tool update-shell`.

### Windows PowerShell

Desde la raíz del repositorio:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

Después abre una nueva PowerShell y verifica:

```powershell
chxchx-tech version
chxchx-tech doctor
```

### Desarrollo local manual

```bash
uv sync --dev
uv tool install --editable .
chxchx-tech version
```

`uv sync --dev` crea o actualiza `.venv` con las dependencias bloqueadas. `uv tool install --editable .` instala el ejecutable `chxchx-tech` apuntando al código local.

### Instalación desde Git

Sustituye `TU_USUARIO` por la URL real del repositorio:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
chxchx-tech version
chxchx-tech doctor
```

La instalación desde Git no incluye herramientas externas como Zellij, Sublime, Docker o los agentes.

## Verificación inicial

Desde este repositorio:

```bash
chxchx-tech version
chxchx-tech doctor .
uv lock --check
uv run pytest -q
uv run python scripts/lab_smoke.py
```

Desde cualquier proyecto que quieras preparar:

```bash
chxchx-tech doctor .
chxchx-tech init --dry-run .
chxchx-tech init .
chxchx-tech status .
```

Si quieres mantener el proyecto compacto, usa:

```bash
chxchx-tech init --minimal .
```

El modo mínimo crea únicamente `.ai/chxchx-tech.toml` y `.ai/memory`. El modo normal también genera reglas, documentación base e integración de OpenCode.

El primer `init --dry-run` muestra los cambios sin escribir archivos. La ejecución real prepara el contexto local y registra el proyecto en `~/.chxchx-tech-lead/projects.json`.

## Flujo recomendado para probar un proyecto

Usa primero un repositorio laboratorio, no un proyecto productivo. El flujo completo es:

```bash
cd /ruta/al/proyecto

# 1. Inspección sin cambios
chxchx-tech doctor .
chxchx-tech init --dry-run .

# 2. Preparación
chxchx-tech init .
chxchx-tech workspace status .

# 3. Revisar .ai/chxchx-tech.toml y añadir procesos si hace falta
$EDITOR .ai/chxchx-tech.toml

# 4. Aprobar localmente los comandos de este repositorio
chxchx-tech workspace trust .

# 5. Revisar el estado antes de ejecutar
chxchx-tech workspace status .
chxchx-tech process list .

# 6. Arrancar y consultar
chxchx-tech workspace start .
chxchx-tech process list .
chxchx-tech resources .

# 7. Entrar a la sesión Zellij
chxchx-tech workspace attach .

# 8. Trabajar con el editor o el agente
chxchx-tech editor open .
chxchx-tech agent start codex --path .
chxchx-tech workspace attach .

# 9. Detener lo gestionado al cambiar de proyecto
chxchx-tech workspace stop .
```

`workspace start` exige que el proyecto esté confiable. Prepara la sesión y arranca procesos `auto_start`, pero no entra de forma interactiva a Zellij. Para ver panes y agentes usa `workspace attach`.

`workspace status`, `process list`, `resources` y los comandos `--dry-run` son de inspección y no deberían iniciar procesos.

## Recetas rápidas del workspace

Para no repetir la secuencia de preparación, agentes y entrada a Zellij:

```bash
# workspace start + agent start --all + workspace attach
chxchx-tech run .

# Preparar todo, pero quedarse en la terminal actual
chxchx-tech run . --no-attach

# Si la sesión activa fue creada antes del header
chxchx-tech run . --recreate

# Atajos individuales
chxchx-tech attach .
chxchx-tech stop .
```

`run` exige trust, prepara el header al crear la sesión, inicia los agentes declarados y entra a
Zellij. Si encuentra una sesión `EXITED`, la elimina solo como estado de terminal abandonado y la
recrea con el layout actual; una sesión activa se reutiliza. Para conservar la sesión al salir,
usa `Ctrl+o` y después `d`; `Ctrl+D` o `exit` cierran el último shell y pueden terminar el
workspace.

El layout completo se aplica al crear la sesión: header arriba, Codex y Claude en columnas
horizontales y una terminal de trabajo adicional. Si la sesión ya existía antes de este cambio,
ejecuta una vez `chxchx-tech run . --recreate`; después `chxchx-tech run .` reutilizará ese layout.
La terminal de trabajo se crea con el shell de inicio (`$SHELL`) en modo login para que la sesión
no termine al crearla en segundo plano.

Cada pane de agente muestra un banner ASCII `CHXCHX TECH · CODEX` o `CHXCHX TECH · CLAUDE`
antes de ejecutar la CLI real. El header general queda arriba y la orientación de los panes de
agentes se controla con `workspace.layout.orientation`.

`--recreate` solo elimina la sesión de terminal Zellij para volver a crearla con el layout
actual; no elimina ni modifica archivos del proyecto.

Si Zellij conserva una sesión `EXITED` y devuelve `Session already exists`, elimínala una vez y
vuelve a ejecutar la receta:

```bash
zellij list-sessions
zellij delete-session --force NOMBRE_DE_SESION
chxchx-tech run . --recreate
```

## Configuración del workspace

`chxchx-tech init` crea una configuración v2 mínima. Los procesos se agregan manualmente en `.ai/chxchx-tech.toml`.

Ejemplo para un proyecto con backend y frontend:

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

[workspace.header]
enabled = true
label = "chxchx-tech"
logo = "◆"
template = "{logo} {label} | {project} | {profile}"

[workspace.layout]
# horizontal = panes lado a lado; vertical = panes apilados
orientation = "horizontal"

[workspace.resources]
warn_memory_percent = 75
critical_memory_percent = 90
warn_swap_percent = 40

[workspace.docker]
enabled = false
compose_file = "compose.yaml"
auto_start = false

[[workspace.processes]]
id = "backend"
label = "Backend"
command = ["npm", "run", "dev"]
cwd = "backend"
auto_start = true
restart = "never"
port = 3001

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

Reglas importantes:

- `command` debe ser una lista de argumentos;
- `cwd` siempre es relativo al proyecto y no puede salir de él;
- `shell = true` solo debe usarse de forma explícita;
- los IDs de procesos y agentes deben ser únicos;
- `workspace.header.template` admite `{logo}`, `{label}`, `{project}` y `{profile}`;
- `workspace.header.logo` acepta texto, emoji o arte ASCII corto;
- `workspace.layout.orientation` acepta `horizontal` (lado a lado) o `vertical` (uno sobre otro);
- `auto_start` no se ejecuta hasta aprobar el trust local;
- no incluir secretos ni valores de `.env` en esta configuración.

Para migrar una configuración v1 existente:

```bash
chxchx-tech init --dry-run .
chxchx-tech init .
```

La migración es validada, idempotente y conserva la configuración v2 existente sin reescribirla innecesariamente.

## Uso correcto del workspace

El workspace tiene tres acciones diferentes:

```bash
# Crea o recupera la sesión, sin entrar en ella
chxchx-tech workspace open .

# Prepara la sesión y arranca procesos con auto_start = true
chxchx-tech workspace start .

# Entra a la sesión Zellij para ver sus panes
chxchx-tech workspace attach .
```

Para iniciar un proceso manualmente:

```bash
chxchx-tech process start api --path .
chxchx-tech process list .
chxchx-tech process stop api --path .
```

Para que `workspace start` arranque un proceso automáticamente, debe tener `auto_start = true` y el proyecto debe estar confiable:

```bash
chxchx-tech workspace trust .
chxchx-tech workspace start .
```

Detener el workspace detiene procesos gestionados y Docker configurado, pero conserva la sesión Zellij para poder recuperarla con `workspace attach`.

## Uso correcto de Codex

Codex debe estar configurado dentro de `.ai/chxchx-tech.toml`:

```toml
[[workspace.agents]]
id = "codex"
command = ["codex"]
cwd = "."
auto_start = false
```

Después de guardar la configuración, el hash cambia y hay que volver a aprobar el proyecto:

```bash
chxchx-tech workspace trust .
chxchx-tech doctor .
chxchx-tech agent start codex --path .
chxchx-tech workspace attach .
```

Para abrir todos los agentes declarados:

```bash
chxchx-tech agent start --all --path .
chxchx-tech workspace attach .
```

Consultar agentes sin iniciar ninguno:

```bash
chxchx-tech agent list --path .
chxchx-tech agent status --path .
```

Los presets permiten iniciar una selección declarada por proyecto:

```toml
[workspace.presets]
default = ["codex", "claude"]
solo-codex = ["codex"]
```

```bash
chxchx-tech agent start --preset solo-codex --path .
```

Para dejar contexto recuperable entre sesiones y agentes:

```bash
chxchx-tech workspace handoff . \
  --summary "Se terminó la integración MCP" \
  --pending "Validar memoria compartida" \
  --validation "chxchx-tech doctor ."
```

El comando actualiza únicamente el bloque administrado de `.ai/HANDOFF.md` y conserva las notas
manuales del archivo.

Cada agente aparece en un pane independiente de la misma sesión Zellij, con un banner ASCII
identificable y luego su CLI normal. Ambos reciben el mismo `cwd`, `AGENTS.md`, `.ai/` y
servidores MCP.

`agent start` crea un pane dentro de la sesión Zellij y devuelve el control a la terminal actual; por eso el último comando es necesario para ver Codex. El agente se inicia con el `cwd` del proyecto y puede leer `AGENTS.md`, `CLAUDE.md` y `.ai/`.

Comprobación previa:

```bash
codex --version
chxchx-tech workspace status .
chxchx-tech agent start codex --path . --dry-run
```

El `--dry-run` debe mostrar una acción equivalente a crear un nuevo pane de Zellij sin ejecutar Codex.

## Comandos disponibles

### Preparación tradicional

```text
chxchx-tech version                    versión instalada
chxchx-tech doctor [PATH]              herramientas y salud del proyecto
chxchx-tech install                    Basic Memory y Serena mediante uv
chxchx-tech init [PATH]                prepara estructura AI y registra proyecto
chxchx-tech setup [PATH]               install + init + integraciones MCP
chxchx-tech status [PATH]              resumen del proyecto
chxchx-tech sync [PATH]                sincroniza bloques administrados
chxchx-tech integrate [PATH]           configura MCP para Claude/Codex/OpenCode
chxchx-tech rollback [PATH]            restaura el último backup gestionado
chxchx-tech projects list              lista proyectos registrados
chxchx-tech projects current           muestra el último proyecto activado
chxchx-tech projects switch ALIAS      suspende el activo y activa otro proyecto
chxchx-tech projects sync              sincroniza proyectos registrados
chxchx-tech run [PATH]                 start + agentes + attach
chxchx-tech attach [PATH]              atajo para entrar a Zellij
chxchx-tech stop [PATH]                atajo para detener procesos y Docker
```

### Workspace y procesos

```text
chxchx-tech workspace status [PATH|ALIAS]  inspección sin ejecutar procesos
chxchx-tech workspace trust [PATH]         aprueba el proyecto localmente
chxchx-tech workspace open [PATH|ALIAS]    prepara sesión y acciones auto_start
chxchx-tech workspace start [PATH|ALIAS]   inicia sesión, procesos y Docker configurado
chxchx-tech workspace stop [PATH|ALIAS]    detiene procesos y Docker del proyecto
chxchx-tech workspace suspend [PATH|ALIAS] detiene procesos y conserva el workspace recuperable
chxchx-tech workspace resume [PATH|ALIAS]  reanuda un workspace suspendido
chxchx-tech workspace attach [PATH|ALIAS]  se adjunta a la sesión existente
chxchx-tech process list [PATH]        muestra procesos y PID
chxchx-tech process start ID --path .  inicia un proceso configurado
chxchx-tech process stop ID --path .   detiene un proceso gestionado
chxchx-tech editor open [PATH]         abre el proyecto en Sublime
chxchx-tech agent start ID --path .    inicia un agente configurado
chxchx-tech agent start --all --path . inicia todos los agentes configurados
chxchx-tech agent start --preset ID --path . inicia un preset de agentes
chxchx-tech agent list --path .       muestra disponibilidad y panes de agentes
chxchx-tech agent status --path .     alias de agent list
chxchx-tech workspace handoff [PATH|ALIAS] actualiza el handoff administrado
chxchx-tech resources [PATH]           muestra RAM, swap, CPU y procesos
```

## Operación multiproyecto

Cada proyecto inicializado queda registrado con un alias derivado de su nombre. Ese alias puede
usarse en los comandos de workspace:

```bash
chxchx-tech projects list
chxchx-tech projects current
chxchx-tech workspace status bokana
chxchx-tech projects switch bokana --no-attach
chxchx-tech projects switch otro-proyecto
```

Al activar cualquier workspace, ChxChx suspende los otros workspaces activos y detiene sus procesos
gestionados. `projects switch` combina esa política con la resolución por alias. El último proyecto
activado se conserva en el registro global. `workspace suspend` y `workspace resume` permiten controlar ese ciclo manualmente. Las sesiones Zellij se
reutilizan o se recuperan al volver a iniciar el workspace; `--recreate` fuerza la reconstrucción
de la sesión cuando se necesita aplicar un layout nuevo.

Para ver las opciones exactas de cualquier comando:

```bash
chxchx-tech --help
chxchx-tech workspace --help
chxchx-tech process start --help
```

## TUI de Textual

El panel interactivo se abre explícitamente así:

```bash
chxchx-tech tui .
```

La TUI incluye pestañas de Resumen, Proyectos, Agentes, Procesos, Recursos y Handoff. Desde Proyectos puedes cambiar el contexto por alias o ruta; desde Agentes puedes revisar disponibilidad de Codex/Claude, sesión Zellij, pane y preset; y desde Handoff puedes leer o actualizar `.ai/HANDOFF.md`.

La vista Resumen muestra el sello ASCII de CHXCHX-DEV. Las acciones no son decorativas: `Iniciar workspace` prepara el workspace y levanta los agentes configurados, `Reintentar agentes` permite recuperarlos si una CLI terminó, la pestaña Procesos permite iniciar o detener un proceso por ID, y Handoff permite editar resumen, pendientes y validación antes de guardarlos.

Si el proyecto no tiene trust, usa el botón `1. Confiar proyecto` del Resumen antes de iniciar. `Preparar` crea la sesión sin entrar a ella; `4. Adjuntar Zellij` entra explícitamente. Las acciones del flujo inicial no requieren escribir texto: los IDs solo se solicitan en las pestañas Agentes y Procesos.

Al adjuntarte a Zellij, la TUI suspende temporalmente su control del terminal y
muestra cómo regresar: pulsa `Ctrl+O`, suelta las teclas y luego pulsa `D`. Al
volver, la TUI restaura el teclado y muestra el resultado. Si Zellij no reconoce la sesión, la operación falla de inmediato
con una instrucción para revisar `zellij list-sessions` y volver a iniciar el
workspace; no queda esperando indefinidamente.

Atajos actuales:

```text
q        salir de la TUI (no detiene el workspace)
Ctrl+P   abrir paleta de comandos
1..7     cambiar de pestaña (7 muestra el sello completo)
r        actualizar
o        abrir/preparar workspace
j        adjuntar a Zellij
y        confiar el proyecto
s        iniciar workspace
t        iniciar workspace y agentes
a        iniciar agentes
c        iniciar agente indicado en la pestaña Agentes
i        iniciar proceso indicado en la pestaña Procesos
k        detener proceso indicado en la pestaña Procesos
u        suspender workspace
v        reanudar workspace
x        detener workspace
h        actualizar handoff
e        abrir Sublime
```

La TUI muestra estado de confianza, sesión, procesos, agentes, recursos y handoff. Todas las acciones pasan por el servicio de workspace y sus adapters; la CLI continúa siendo la interfaz recomendada para scripts y CI.

## MCP, Basic Memory y Serena

Instalar herramientas gestionadas:

```bash
chxchx-tech install
```

Preparar todo el proyecto y revisar antes la integración:

```bash
chxchx-tech setup --dry-run .
chxchx-tech setup .
```

Integrar un cliente concreto:

```bash
chxchx-tech integrate --dry-run --client claude .
chxchx-tech integrate --client claude .

chxchx-tech integrate --dry-run --client codex .
chxchx-tech integrate --client codex .

chxchx-tech integrate --dry-run --client opencode .
chxchx-tech integrate --client opencode .
```

ChxChx no guarda tokens, no copia `.env` y no instala automáticamente clientes que tienen su propio instalador.

Para publicar un repositorio preparado, revisa siempre `git status` y `git diff --cached`.
El `.gitignore` excluye `.ai/`, `.serena/`, memorias, SQLite, logs, credenciales,
material criptográfico y reglas generadas de agentes. No reemplaza una auditoría:
si un secreto llegó a entrar en Git, debe revocarse y retirarse también del historial.

### Memoria compartida entre Codex y Claude

Ejecuta la integración para ambos clientes:

```bash
chxchx-tech integrate --client codex .
chxchx-tech integrate --client claude .
```

ChxChx registra un único proyecto de Basic Memory usando `memory_project` y `.ai/memory`. Las configuraciones MCP de Codex y Claude reciben explícitamente:

```text
basic-memory mcp --project NOMBRE_DEL_PROYECTO
```

Así ambos agentes consultan y escriben el mismo conocimiento. En modo local, Basic Memory usa SQLite para su índice y configuración, mientras las notas fuente se conservan en `.ai/memory` como Markdown portable. No se debe escribir directamente en la base SQLite desde ChxChx.

Si esos servidores MCP ya estaban registrados antes de preparar la memoria compartida, actualízalos con la CLI oficial después de revisar el `--dry-run`:

```bash
chxchx-tech integrate --dry-run --refresh --client codex .
chxchx-tech integrate --refresh --client codex .
chxchx-tech integrate --dry-run --refresh --client claude .
chxchx-tech integrate --refresh --client claude .
```

`--refresh` elimina y vuelve a registrar únicamente los servidores `basic-memory` y `serena` administrados por ChxChx; no toca otros servidores MCP.

Para comprobar el proyecto de memoria:

```bash
bm project list
bm project info NOMBRE_DEL_PROYECTO --json
```

## Herramientas externas

Comprueba su disponibilidad con:

```bash
zellij --version
subl --version
docker --version
docker compose version
codex --version
claude --version
opencode --version
```

`chxchx-tech doctor` las clasifica como disponibles o ausentes. Una herramienta opcional ausente no invalida la instalación base. Si Zellij no está disponible, el workspace utiliza el fallback de subprocess con menos persistencia de sesión.

## Seguridad y rollback

La confianza se guarda fuera del repositorio en:

```text
~/.chxchx-tech-lead/trusted_projects.json
```

Se asocia a la ruta y al hash de `.ai/chxchx-tech.toml`; si cambia la configuración, debe aprobarse de nuevo. El estado y los logs también viven fuera del proyecto:

```text
~/.chxchx-tech-lead/projects.json
~/.chxchx-tech-lead/workspace-state.json
~/.chxchx-tech-lead/logs/
```

Antes de modificar archivos administrados:

```bash
chxchx-tech init --dry-run .
chxchx-tech setup --dry-run .
```

Para recuperar el último backup de `init`/`sync`:

```bash
chxchx-tech rollback .
```

El rollback restaura archivos gestionados; no intenta revertir procesos del sistema ni cambios hechos manualmente fuera del alcance de ChxChx.

## Solución de problemas

### `chxchx-tech: command not found`

```bash
uv tool list
uv tool update-shell
```

Abre una terminal nueva. En Linux/macOS, confirma que `~/.local/bin` esté en `PATH`.

### `uv: command not found`

Linux/macOS:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://astral.sh/uv/install.ps1 | iex"
```

### `No existe el agente configurado: codex`

La detección de `codex --version` no configura automáticamente el agente. Añade esta sección a `.ai/chxchx-tech.toml`:

```toml
[[workspace.agents]]
id = "codex"
command = ["codex"]
cwd = "."
auto_start = false
```

Después vuelve a confiar el proyecto y ejecútalo dentro de la sesión:

```bash
chxchx-tech workspace trust .
chxchx-tech agent start codex --path .
chxchx-tech workspace attach .
```

### Codex se inicia, pero no lo veo

`agent start` crea el pane y termina; no entra automáticamente a Zellij. Usa:

```bash
chxchx-tech workspace attach .
```

También puedes comprobar que la sesión existe con:

```bash
zellij list-sessions
```

Si `workspace start` o `workspace attach` indica que Zellij no reconoce la
sesión, el comando termina inmediatamente y el estado queda en `ERROR`; no
continúa esperando. Revisa la salida de `zellij list-sessions` y vuelve a
ejecutar `chxchx-tech workspace start .`. Si aparece como `EXITED`, elimina la
sesión indicada con `zellij delete-session --force NOMBRE` y vuelve a iniciar.

### Quiero abrir Codex y Claude juntos

Confirma que ambos bloques existan en `.ai/chxchx-tech.toml` y ejecuta:

```bash
chxchx-tech workspace trust .
chxchx-tech integrate --client codex .
chxchx-tech integrate --client claude .
chxchx-tech agent start --all --path .
chxchx-tech workspace attach .
```

Si el proyecto fue inicializado antes de que se añadieran los agentes por defecto, agrega sus bloques manualmente en `.ai/chxchx-tech.toml`.

### Serena muestra `Error loading configuration` o errores de estadísticas

El binario instalado no garantiza que exista una sesión MCP activa. Comprueba que el proyecto
tenga `.serena/project.yml`, registra sus servidores y reinicia el agente que tenía abierta la
sesión anterior:

```bash
serena --version
serena project health-check .
chxchx-tech integrate --refresh --client claude .
claude mcp list
```

Después cierra y vuelve a abrir Claude/Codex. El dashboard de Serena muestra datos de la sesión
MCP actual; si quedó abierto de una ejecución anterior puede mostrar errores aunque la
configuración del proyecto sea válida.

### `workspace start` solo muestra la sesión

Eso es correcto si no hay procesos con `auto_start = true`. `workspace start` no inventa comandos ni inicia procesos no configurados. Define los procesos en `.ai/chxchx-tech.toml`, marca los que deban arrancar automáticamente y vuelve a ejecutar:

```bash
chxchx-tech workspace start .
chxchx-tech process list .
```

### El workspace no inicia

Ejecuta, en este orden:

```bash
chxchx-tech doctor .
chxchx-tech workspace status .
chxchx-tech process list .
```

Si aparece `no confiable`:

```bash
chxchx-tech workspace trust .
```

Si aparece un `cwd` inválido, corrige la ruta relativa en `.ai/chxchx-tech.toml`. Si el ejecutable no existe, instálalo según su documentación o cambia `command`.

### Zellij no está instalado

El workspace puede usar el fallback de subprocess, pero no tendrá sesiones persistentes completas. Instala Zellij con el gestor de paquetes de tu sistema o desde su documentación oficial y vuelve a comprobar:

```bash
zellij --version
chxchx-tech doctor .
```

### Quiero detener todo lo gestionado

```bash
chxchx-tech workspace stop .
```

Esto no ejecuta `docker compose down`, no borra contenedores y no mata procesos que no hayan sido registrados por ChxChx.
