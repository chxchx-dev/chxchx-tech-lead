```text
╔════════════════════════════════════════════════════════════════════════════════════════════╗
║                                                                                            ║
║   ██████╗██╗  ██╗██╗  ██╗ ██████╗██╗  ██╗██╗  ██╗      ████████╗███████╗ ██████╗██╗  ██╗   ║
║  ██╔════╝██║  ██║╚██╗██╔╝██╔════╝██║  ██║╚██╗██╔╝      ╚══██╔══╝██╔════╝██╔════╝██║  ██║   ║
║  ██║     ███████║ ╚███╔╝ ██║     ███████║╚███╔╝ █████╗   ██║   █████╗  ██║     ███████║    ║
║  ██║     ██╔══██║ ██╔██╗ ██║     ██╔══██║ ██╔██╗ ╚════╝   ██║   ██╔══╝  ██║     ██╔══██║   ║
║  ╚██████╗██║  ██║██╔╝ ██╗╚██████╗██║  ██║██╔╝ ██╗         ██║   ███████╗╚██████╗██║  ██║   ║
║   ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝         ╚═╝   ╚══════╝ ╚═════╝╚═╝  ╚═╝   ║
║                                                                                            ║
║                        ██╗     ███████╗ █████╗ ██████╗                                     ║
║                        ██║     ██╔════╝██╔══██╗██╔══██╗                                    ║
║                        ██║     █████╗  ███████║██║  ██║                                    ║
║                        ██║     ██╔══╝  ██╔══██║██║  ██║                                    ║
║                        ███████╗███████╗██║  ██║██████╔╝                                    ║
║                        ╚══════╝╚══════╝╚═╝  ╚═╝╚═════╝                                     ║
║                                                                                            ║
║                         CHXCHX-TECH-LEAD                                                   ║
║                                                                                            ║
║                ────────────────────────────────────────                                    ║
║ 		  Bootstrapper y control plane local para repositorios de 							 ║
║				desarrollo asistido por agentes.											 ║
║                ────────────────────────────────────────                                    ║
║                                                                                            ║
║                 by [@chxchx-dev](https://github.com/chxchx-dev)            			     ║
║                                                                                            ║
╚════════════════════════════════════════════════════════════════════════════════════════════╝
```
ChxChx Tech Lead prepara el contexto de un proyecto, conecta herramientas externas y administra un workspace ligero desde la terminal. No reemplaza a Git, Zellij, Sublime, Codex, Claude ni OpenCode: los detecta y los orquesta mediante adapters.

## Funciones

La versión pública estable `0.2.0` incluye preparación idempotente de proyectos, perfiles y detección de stack, backups y rollback, integraciones MCP, registro multiproyecto, diagnóstico y smoke test. Terminal Workspace todavía no forma parte de esa release.

La rama `dev` añade el Terminal Workspace. El PR pasó la matriz automatizada de Linux, macOS y Windows (Python 3.11–3.13); la validación manual de release y el uso sostenido en proyectos reales siguen antes de publicar una versión nueva:

- configuración `.ai/chxchx-tech.toml` v2 y migración v1 → v2;
- trust local para impedir ejecutar comandos de repositorios no aprobados;
- Process Manager con estado persistente, logs rotativos y protección contra PIDs ajenos;
- adapters para Zellij, fallback de subprocess, Sublime, agentes CLI y Git;
- comandos CLI de workspace, procesos, agentes, editor y recursos;
- dashboard con Textual y operación multiproyecto mediante sesiones recuperables;
- Skill Registry local con búsqueda, consulta de instrucciones y recomendaciones por stack.

El catálogo contiene 17 skills generales y para Python, TypeScript, React/Next.js,
NestJS, .NET, React Native, PostgreSQL, Prisma, Redis, Docker y diseño UI/UX de
producto. La skill UI/UX se recomienda para proyectos React, Next.js y React
Native y forma parte de sus Tech Packs.
Las recomendaciones no cambian el proyecto. Habilita de forma explícita las
skills que quieres añadir al contexto:

```bash
chxchx-tech skill list
chxchx-tech skill search python
chxchx-tech skill info python-engineering
chxchx-tech skill recommend .
chxchx-tech skill enable nextjs .
chxchx-tech skill sync . --dry-run
chxchx-tech skill sync .
chxchx-tech pack list
chxchx-tech pack detect .
chxchx-tech pack apply-detected . --dry-run
chxchx-tech pack apply-detected .
chxchx-tech skill status .
```

La selección por proyecto se guarda en `.ai/chxchx-skills.toml`; `skill sync`
genera las instrucciones activas en `.ai/SKILLS.md` y añade una referencia
administrada a `AGENTS.md`. Usa `skill disable NOMBRE .` y vuelve a sincronizar
para retirar una skill del contexto.
No instalas cada skill por separado: las 17 guías y 11 Tech Packs vienen con
ChxChx. `skill list` informa el total de guías; `pack list` muestra cada pack,
cuántas skills incluye y sus nombres; `pack info NOMBRE` consulta uno en
particular. Así puedes ver exactamente qué se añadirá antes de aplicarlo.

`pack detect` explica qué stack o lenguaje coincidió. `pack apply-detected`
añade de una vez la unión de los packs compatibles, sin escribir sus nombres;
`--dry-run` permite previsualizar. La selección queda guardada por proyecto,
`skill status` cuenta cuáles están activas y después `skill sync .` carga esa
selección sin volver a nombrar las skills. Son instrucciones Markdown para los
agentes, no paquetes ejecutables ni utilidades instaladas individualmente.

## Requisitos

Obligatorios:

- Python `>=3.11`;
- Git;
- `uv`;
- una terminal.

Para usar todas las funciones del workspace, instala también según necesidad:

- Zellij para sesiones persistentes;
- Sublime Text para editar;
- Codex CLI, Claude Code u OpenCode para agentes.

ChxChx instala únicamente las herramientas gestionadas por él: Basic Memory y Serena. Los clientes de escritorio, Zellij y agentes mantienen su instalación oficial separada.

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

### Prueba rápida de ChxChx Studio en Fedora

El instalador descarga el paquete público de la pre-release Fedora más reciente de `dev`, instala las bibliotecas Qt de ejecución y deja Studio en `~/.local/opt/chxchx-studio/`. También instala el bridge CLI en un entorno aislado para esta versión y crea `~/.local/bin/chxchx-studio-dev`; no reemplaza otro `chxchx-tech` global. Requiere Fedora x86_64, conexión a GitHub, `sudo` y que CI termine correctamente.

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio-fedora.sh | bash
chxchx-studio-dev /ruta/al/proyecto
```

Es una compilación de desarrollo para pruebas, no una versión estable. Después de cada push a `dev`, espera a que los jobs Fedora y de publicación terminen; el enlace siempre instala el paquete más reciente publicado. Para quitarla, elimina el launcher y la carpeta `~/.local/opt/chxchx-studio/dev-<commit>` indicada al instalar. Las dependencias Qt instaladas mediante DNF se administran aparte con `dnf remove` si ya no las necesita otra aplicación.

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
uv run chxchx-tech version
uv run chxchx-tech tui .
```

`uv sync --dev` crea o actualiza `.venv` con las dependencias bloqueadas, incluida Textual para la TUI. `uv run chxchx-tech` ejecuta el código de este checkout, así que los cambios quedan disponibles al volver a correr el comando. No hace falta instalar el CLI globalmente para desarrollar.

### Instalación desde Git

La instalación estable publicada sigue en `v0.2.0`:

```bash
uv tool install "git+https://github.com/chxchx-dev/chxchx-tech-lead.git@v0.2.0"
chxchx-tech version
chxchx-tech doctor
```

La instalación desde Git no incluye herramientas externas como Zellij, Sublime o los agentes.

### Actualizar una instalación existente para probar `dev`

El build actual de la rama `dev` se identifica como `0.5.0.dev0`. No es una
release estable; úsalo para evaluar Terminal Workspace y registra los
resultados localmente. La carpeta `docs/` contiene notas de trabajo locales y
no forma parte del repositorio publicado.

Cierra la TUI. Si el CLI instalado ya ofrece el comando, suspende primero el
workspace para detener los procesos gestionados y poder reanudarlo después:

```bash
chxchx-tech workspace suspend .
```

Luego, tanto en Linux/macOS como en Windows PowerShell, reemplaza la instalación
global de ChxChx por la de `dev`:

```bash
uv tool install --force "git+https://github.com/chxchx-dev/chxchx-tech-lead.git@dev"
chxchx-tech version
chxchx-tech doctor .
chxchx-tech workspace status .
```

`version` debe mostrar `chxchx-tech-lead 0.5.0.dev0`. Si suspendiste el workspace,
reanúdalo y continúa la prueba:

```bash
chxchx-tech workspace resume .
```

`--force` reemplaza el entorno de la herramienta ChxChx. La actualización no
modifica la configuración `.ai/` de tus proyectos ni actualiza Basic Memory,
Serena, Zellij, Sublime o los clientes de agentes. No ejecutes de nuevo los
scripts de instalación general para actualizar solo ChxChx.

Si ya lo instalaste en modo editable desde un clon, actualiza ese checkout limpio
en la rama `dev` con `git pull --ff-only origin dev` y reinstala desde su raíz:

```bash
uv tool install --force --editable .
```

Consulta la [guía oficial de herramientas de uv](https://docs.astral.sh/uv/guides/tools/) para más detalles sobre la reinstalación forzada.

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

# 8. Configurar Sublime y trabajar con el editor o el agente
chxchx-tech editor setup . --dry-run
chxchx-tech editor setup .
chxchx-tech editor open .
chxchx-tech agent start codex --path .
chxchx-tech workspace attach .

# 9. Detener lo gestionado al cambiar de proyecto
chxchx-tech workspace stop .
```

`workspace start` exige que el proyecto esté confiable. Prepara la sesión y arranca procesos `auto_start`, pero no entra de forma interactiva a Zellij. Para ver panes y agentes usa `workspace attach`.

Puedes confiar el proyecto antes de inicializarlo desde la TUI o con
`chxchx-tech workspace trust`. Esto no habilita el inicio hasta que exista una
configuración válida. La confianza queda vinculada al contenido de
`.ai/chxchx-tech.toml`; si se crea o cambia después, vuelve a confiar el
proyecto antes de iniciar procesos.

`workspace status`, `process list`, `resources` y los comandos `--dry-run` son de inspección y no deberían iniciar procesos.
La acción «Preparar proyecto de Sublime» también está disponible desde la paleta de comandos de la TUI.

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

La sesión ofrece dos pestañas: `Terminales`, con el header y el shell interactivo, y `Agentes`,
con el header y el panel de uso arriba, más los agentes debajo. Si falta `Agentes` en una sesión
activa, se crea al adjuntarse el cliente y queda enfocada. Los agentes se muestran lado a lado
con `workspace.layout.orientation = "horizontal"` y apilados con `"vertical"`. Si la sesión ya
tiene una pestaña `Agentes` creada con un layout anterior, ejecuta una vez
`chxchx-tech run . --recreate`; después se reutilizará. El shell de trabajo usa `$SHELL` en modo
interactivo.

Cada pane de agente muestra un banner ASCII `CHXCHX TECH · CODEX` o `CHXCHX TECH · CLAUDE`
antes de ejecutar la CLI real. El header general queda arriba y la orientación de los panes de
agentes se controla con `workspace.layout.orientation`.

La confianza registrada por ChxChx Tech Lead permite preparar el workspace. Codex y Claude Code
pueden mostrar además su propia confirmación de confianza la primera vez que abren esa carpeta;
respóndela dentro del pane del agente para continuar.

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
warn_memory_percent = 70
critical_memory_percent = 85
warn_swap_percent = 40
max_agents = 2

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

Para ahorrar tokens, abre una conversación limpia y haz que el agente reconstruya el contexto guardado del proyecto:

```bash
chxchx-tech agent start codex --new-chat --path .
# o para Codex y Claude configurados:
chxchx-tech agent start --all --new-chat --path .
chxchx-tech workspace attach .
```

`--new-chat` está disponible para las CLIs oficiales `codex` y `claude`. Al terminar cada sesión de esos agentes, ChxChx revisa los transcripts locales del proyecto y guarda solicitudes explícitas de recordar en `.ai/memory/PROJECT_MEMORY.md`; al iniciar un chat nuevo las recupera sin duplicarlas y las pasa al contexto inicial. Omite preguntas de recuperación, conversaciones de otros proyectos y mensajes que parezcan contener secretos; no carga transcripciones completas. También incluye el valor exacto de `memory_project` desde `.ai/chxchx-tech.toml`: las herramientas Basic Memory deben usar ese nombre en el argumento `project`, sin reutilizar IDs de otros proyectos ni el ámbito predeterminado. Las frases se buscan primero en el archivo local; si la memoria no está configurada, el agente no debe hacer búsquedas globales. La captura depende de que el transcript local del proveedor exista y pueda leerse al terminar la sesión; no puede recuperar mensajes ausentes de esos archivos. `.ai/` es local a cada checkout y está excluido de Git. Los agentes distintos de Codex/Claude siguen pudiendo iniciarse normalmente, pero aún no reciben esta captura automática.

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

Detener el workspace detiene los procesos gestionados y conserva la sesión Zellij para poder recuperarla con `workspace attach`.

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
chxchx-tech stop [PATH]                atajo para detener procesos gestionados
```

### Workspace y procesos

```text
chxchx-tech workspace status [PATH|ALIAS]  inspección sin ejecutar procesos
chxchx-tech workspace trust [PATH]         aprueba el proyecto localmente
chxchx-tech workspace open [PATH|ALIAS]    prepara sesión y acciones auto_start
chxchx-tech workspace start [PATH|ALIAS]   inicia sesión y procesos configurados
chxchx-tech workspace stop [PATH|ALIAS]    detiene procesos gestionados
chxchx-tech workspace suspend [PATH|ALIAS] detiene procesos y conserva el workspace recuperable
chxchx-tech workspace resume [PATH|ALIAS]  reanuda un workspace suspendido
chxchx-tech workspace attach [PATH|ALIAS]  se adjunta a la sesión existente
chxchx-tech process list [PATH]        muestra procesos y PID
chxchx-tech process start ID --path .  inicia un proceso configurado
chxchx-tech process stop ID --path .   detiene un proceso gestionado
chxchx-tech editor open [PATH]         abre el proyecto en Sublime
chxchx-tech editor setup [PATH]        genera proyecto Sublime local y lo abre
chxchx-tech editor setup [PATH] --no-open solo genera el proyecto Sublime
chxchx-tech agent start ID --path .    inicia un agente configurado
chxchx-tech agent start ID --new-chat --path . inicia conversación nueva con contexto persistido (Codex/Claude)
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

La TUI incluye pestañas de Resumen, Trabajo, Proyectos, Skills, Más y Memoria. Desde Skills puedes buscar el catálogo, revisar recomendaciones y Tech Packs, habilitar o deshabilitar skills, aplicar packs y sincronizar el contexto del proyecto. Desde Proyectos puedes cambiar el contexto por alias o ruta; desde Agentes puedes revisar disponibilidad de Codex/Claude, sesión Zellij, pane y preset; Handoff permite leer o actualizar `.ai/HANDOFF.md`; y Memoria muestra, filtra y abre las notas persistentes del proyecto activo en `.ai/memory`.

En Agentes, `Nuevo chat + contexto` abre un pane nuevo para el ID indicado y solicita a Codex o Claude reconstruir el contexto desde los archivos y Basic Memory del proyecto activo. El panel `Checkpoint automático` verifica que las reglas de guardado estén presentes, que Basic Memory aparezca configurado para cada cliente y muestra la nota más reciente encontrada. Es un diagnóstico estático más evidencia de escritura; no puede certificar que el modelo haya guardado cada turno ni que el servidor MCP responda en vivo.

El RAM Governor muestra el uso general de memoria, swap y CPU junto con el consumo estimado de procesos del proyecto. Antes de iniciar agentes desde la TUI, advierte si se superan los umbrales configurados o el límite de agentes; puedes cancelar o continuar explícitamente. En `agent start`, `run` y `projects switch`, el CLI informa el riesgo y requiere `--force` para iniciar; `--dry-run` solo muestra el aviso. Nunca detiene procesos por su cuenta. Configúralo en `[workspace.resources]` con `warn_memory_percent`, `critical_memory_percent`, `warn_swap_percent` y `max_agents`. Recursos también compara proyectos registrados. La estimación por proyecto usa PID principales declarados; no incluye procesos hijos ni mide los agentes dentro de panes Zellij.

La TUI agrupa la operación habitual en **Inicio**, **Trabajo** y **Proyectos**.
La pestaña **Skills** da acceso directo al catálogo y sus packs. **Más** contiene configuración, recursos, historial, notas y ayuda.
En Inicio, inicializa y confía el proyecto si hace falta. Después puedes abrir
la shell del proyecto o entrar directamente a la pestaña de agentes con los
botones **Abrir terminal** y **Abrir agentes**, o las teclas `T` y `A`. La paleta
permite buscar acciones menos frecuentes.

Al adjuntarte a Zellij, la TUI suspende temporalmente su control del terminal y
muestra cómo regresar: pulsa `Ctrl+O`, suelta las teclas y luego pulsa `D`. Al
volver, la TUI restaura el teclado y muestra el resultado. Las operaciones
largas se ejecutan en segundo plano para mantener disponible la navegación.

Atajos principales:

```text
q        salir de la TUI (no detiene el workspace)
Ctrl+P   abrir paleta de comandos
t        abrir la terminal del proyecto
a        preparar y abrir agentes
1..4     navegar Inicio, Trabajo, Proyectos y Más
r        actualizar
```

La TUI muestra estado de confianza, sesión, procesos, agentes, recursos y handoff. Todas las acciones pasan por el servicio de workspace y sus adapters; la CLI continúa siendo la interfaz recomendada para scripts y CI.

## Aplicación nativa en desarrollo

ChxChx Studio es una aplicación de escritorio en C++20 y Qt 6 Widgets. Incluye navegación del proyecto, pestañas de edición, guardado atómico y lecturas asíncronas mediante el CLI instalado. Las acciones están agrupadas en los menús Archivo, Edición, Ver, Navegar, Herramientas y Ayuda; la barra superior conserva accesos rápidos con iconos y tooltips, e incluye botones marcados para mostrar u ocultar Explorador, Secciones/acciones y Actividad. `Edición → Configurar atajos` permite cambiar las combinaciones, valida duplicados y guarda las preferencias del usuario. El explorador muestra iconos del sistema para archivos y carpetas, con tamaño legible. Su campo superior filtra nombres visibles; `Navegar → Buscar en el proyecto` (`Ctrl+Shift+F`) busca nombres/rutas indexados o contenido en segundo plano. El índice omite directorios generados comunes y limita el catálogo a 100.000 archivos; el escaneo de contenido omite binarios y archivos mayores de 1 MiB, con máximo de 64 MiB leídos y 100 coincidencias por consulta. Al elegir una coincidencia se abre el archivo en la línea correspondiente. `Navegar → Símbolos del archivo actual` (`Ctrl+Shift+O`) lista declaraciones comunes de Python, C/C++, JavaScript/TypeScript, Java y C# y salta a su línea; es un escáner de declaraciones, no resolución semántica. Studio usa el tema de iconos del sistema y recurre a iconos estándar de Qt cuando no encuentra uno. En el árbol, un clic despliega carpetas y abre archivos. Proyecto permite dar trust, iniciar/reanudar, suspender y detener procesos del workspace; Proyectos lista las rutas registradas, permite dar trust a un destino y cambiar al proyecto elegido sin adjuntar la terminal. Agentes abre el comando CLI configurado dentro de una pestaña central junto a los archivos, con una terminal VT interactiva sobre PTY/ConPTY. Antes de abrir agentes, Studio vuelve a verificar trust y RAM mediante un preflight de solo lectura; cualquier advertencia del RAM Governor requiere confirmación explícita. Studio usa el wrapper compartido para conservar la instrumentación del agente, y “Nueva sesión” agrega el prompt de recuperación de contexto disponible para Codex y Claude Code. `Terminal +` y `Ctrl+Shift+T` abren una shell integrada en una pestaña del proyecto; la rueda y `Shift+PageUp/PageDown` recorren hasta 5000 líneas de scrollback, y `Ctrl+End` vuelve al final. La paleta puede adjuntar Zellij dentro de una pestaña PTY tras preview y confirmación; el emulador externo sigue disponible. `Herramientas → Diff Git del archivo actual` muestra en solo lectura el cambio rastreado del archivo respecto de `HEAD`, con salida acotada a 2 MiB; no incluye archivos sin seguimiento ni cambios que sigan solo en el buffer. La actividad de operaciones vive en un panel independiente que se puede mostrar desde Ver o desde la barra, y se oculta al iniciar para dar más espacio al editor y las terminales. La interfaz, los controles y las solicitudes de permiso pertenecen al CLI del agente; Studio administra pestañas y cierre de sesiones. Se admiten comandos configurados como listas de argumentos; los comandos de shell personalizados no se pueden embeber. Procesos permite iniciar/detener procesos configurados. Skills muestra catálogo, selección activa y recomendaciones; permite habilitar/deshabilitar skills y sincronizar contexto. Tech Packs muestra composición y detección; permite aplicar un pack o todos los detectados y sincronizar. Handoff permite revisar el estado, editar resumen/pendiente/validación y guardar con preview y confirmación. Memoria permite buscar y leer notas locales en solo lectura. Chats permite filtrar el historial local de Codex y Claude Code y abrir transcripciones; Errores muestra los registros recientes filtrados al proyecto. Configuración permite previsualizar y ejecutar setup, init completo/mínimo, instalación de herramientas e integración MCP; Doctor es de solo lectura. Guía incluye un recorrido paso a paso con botones que llevan a Configuración, Skills, Proyecto y Agentes; Marca y la paleta filtrable de comandos también están disponibles en la navegación. Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs y Recursos muestran su estado en el área central; Skills distingue habilitadas/recomendadas y Agentes muestra disponibilidad y sesiones activas de Studio.

Las vistas Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills y Tech Packs consultan el contrato JSON local y versionado de `chxchx-tech bridge status .`. Recursos usa `chxchx-tech bridge resources .` para mostrar una muestra global de RAM/swap/CPU y consumo agregado de procesos gestionados para cada proyecto registrado. Handoff y Memoria usan lecturas bajo demanda con `chxchx-tech bridge handoff PATH` y `chxchx-tech bridge memory PATH --query TEXTO`. Chats usa `bridge chats` para buscar y carga cada transcripción seleccionada con `bridge conversation`; Errores consulta `bridge errors`. Son lecturas locales y no cambian sesiones ni la caché de errores.

La memoria resiliente entre sesiones vive por proyecto en `.ai/memory/PROJECT_MEMORY.md`. El wrapper guarda las solicitudes explícitas al terminar la sesión del agente y ChxChx las recupera al iniciar un chat nuevo, siempre desde transcripts locales del mismo proyecto. El prompt incluye el nombre exacto de Basic Memory configurado para ese checkout y prohíbe usar IDs o ámbitos predeterminados de otros proyectos. No se guardan conversaciones completas, preguntas de recuperación ni mensajes que parezcan contener secretos. Esta nota persiste al cerrar y abrir Studio en el mismo checkout; `.ai/` está excluido de Git, así que no se copia a otro clon automáticamente. Si Basic Memory está configurado para el proyecto, usa esa carpeta local como su directorio de notas.

Inicio incluye un panel central de acciones rápidas para confiar el proyecto, iniciar/reanudar, suspender o detener el workspace, abrir una terminal, preparar el proyecto, sincronizar skills y abrir agentes o sesiones nuevas. Estas acciones conservan la previsualización, confirmación y preflight de trust/RAM de sus flujos compartidos; ya no requieren abrir primero el panel lateral de Secciones y acciones.

La paleta de comandos también puede adjuntar a una sesión Zellij dentro de una pestaña PTY/libvterm integrada; conserva una segunda opción para abrir ese attach en el emulador externo como fallback.

Studio carga los logos de `assets/` como recursos Qt: la variante mínima identifica la aplicación y la ventana; el logo completo aparece en la vista Marca.

El editor usa el widget Qt oficial de Scintilla con Lexilla para resaltado de sintaxis; las sesiones interactivas usan libvterm para renderizar ANSI/VT, responder consultas del terminal y reenviar teclado, teclas de función, pegado desde el portapapeles, cambios de tamaño y mouse/rueda cuando el agente los solicita. CMake descarga revisiones fijadas de esos proyectos la primera vez que se configura; esa configuración inicial requiere acceso a GitHub. Las versiones y avisos de licencia están en [native/THIRD_PARTY.md](native/THIRD_PARTY.md).

Requisitos para compilar: CMake 3.21+, Ninja (opcional) y Qt 6.4+ con Core, Widgets y Core5Compat. En Fedora instala `qt6-qtbase-devel` y `qt6-qt5compat-devel`; en Ubuntu instala los paquetes de desarrollo Qt6 equivalentes.

Para el ciclo diario de desarrollo en macOS, Linux o WSL, este comando configura, recompila y abre Studio con el checkout actual como proyecto:

```bash
./scripts/dev-studio.sh
```

También puedes abrir otro proyecto pasándole su ruta; `CHXCHX_BUILD_DIR` permite elegir otra carpeta de build:

```bash
./scripts/dev-studio.sh /ruta/al/proyecto
CHXCHX_BUILD_DIR=/tmp/chxchx-build ./scripts/dev-studio.sh
```

El flujo manual y las pruebas nativas siguen disponibles:

```bash
cmake -S native -B build/native -DCHXCHX_BUILD_TESTS=ON
cmake --build build/native
ctest --test-dir build/native --output-on-failure
./build/native/chxchx-studio .
```

En Windows y macOS se ejecuta el binario generado desde `build/native` con el proyecto como argumento. El editor Scintilla tiene UTF-8, guardado atómico, números de línea, búsqueda en archivo y lexers para C/C++, Python, JavaScript/TypeScript, JSON, YAML, TOML, Markdown, HTML/XML, CSS, SQL, Bash y CMake. `Terminal +` o `Ctrl+Shift+T` abre una shell integrada con PTY/ConPTY y libvterm; el scrollback se navega con la rueda, `Shift+PageUp/PageDown`, `Ctrl+Home` (inicio) y `Ctrl+End` (final). Attach a Zellij puede usar la terminal integrada; el launcher externo requiere Windows Terminal en Windows, un emulador compatible instalado en Linux o Terminal.app en macOS. Las sesiones de agentes configuradas como argumentos también se abren dentro de Studio. Studio recuerda hasta 20 archivos recientes dentro del proyecto y restaura archivos limpios y grupos divididos al cerrar/reabrir. Siguen pendientes LSP, QA interactivo de todos los flujos y verificación de los paquetes en los runners objetivo.

CPack genera un ZIP para Windows, DMG para macOS y TGZ para Linux. Para crear uno localmente, agrega `-DCMAKE_BUILD_TYPE=Release` al configurar y ejecuta `cpack --config build/native/CPackConfig.cmake -C Release -B build/native/packages`. Los jobs de CI con Qt SDK generan esos paquetes y publican cada uno como artefacto de la ejecución. El TGZ generado con Qt instalado desde los paquetes de Fedora/Ubuntu puede depender de las bibliotecas Qt del sistema; para distribuirlo, usa el artefacto CI creado desde Qt SDK y valida las dependencias de la plataforma destino. Los avisos de Scintilla, Lexilla y libvterm se incluyen en el paquete.

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

`chxchx-tech doctor [PATH]` también revisa si Basic Memory y Serena están configurados
para ese proyecto en Claude Code y Codex, y advierte si solo encuentra una configuración
global que podría compartirse con otros repositorios. El chequeo es estático: no inicia
agentes ni servidores MCP, no valida conectividad en vivo y no muestra variables de entorno.
En Codex, la configuración MCP local solo se carga cuando el proyecto es de confianza.

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

Esto no elimina datos del proyecto ni mata procesos que no hayan sido registrados por ChxChx.
