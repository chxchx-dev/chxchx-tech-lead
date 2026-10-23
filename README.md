# ChxChx Tech Lead

ChxChx Tech Lead prepara el contexto de un proyecto, conecta herramientas externas y administra un workspace ligero desde la terminal. ChxChx Studio es el editor integrado; Git, Zellij y los agentes mantienen sus propias herramientas y ChxChx los conecta mediante adapters. Sublime sigue disponible como editor externo opcional durante la transición.

**ChxChx Studio** es su aplicación nativa de escritorio en C++20/Qt 6 para
Windows, macOS y Linux. Puedes [instalar el preview sin clonar el repositorio](#instalar-chxchx-studio-sin-clonar-el-repositorio); el instalador descarga el paquete correcto y su bridge CLI aislado.

## Empieza aquí: Studio o TUI

Ambas interfaces trabajan con la misma configuración y los mismos servicios del
proyecto. Elige Studio para editar código en una ventana, la TUI para trabajar
desde la terminal o la CLI para automatizar comandos.

| Quiero… | Abro… |
| --- | --- |
| Editar archivos y manejar el proyecto en una ventana | [ChxChx Studio](#usar-chxchx-studio) |
| Operar el proyecto desde una terminal interactiva | [La TUI](#usar-la-tui) |
| Ejecutar una acción puntual o automatizarla | [Los comandos CLI](#comandos-disponibles) |

### Primera vez en Studio

1. [Instala Studio](#instalar-chxchx-studio-sin-clonar-el-repositorio) o
   [compílalo desde el checkout](#compilar-studio-desde-el-checkout).
2. Abre tu proyecto pasando su ruta al launcher:

   ```bash
   chxchx-studio-dev /ruta/al/proyecto
   ```

   En Windows PowerShell:

   ```powershell
   & "$env:LOCALAPPDATA\Programs\ChxChxStudio\chxchx-studio-dev.cmd" "C:\ruta\al\proyecto"
   ```

3. En **Inicio**, pulsa **Preparar y entrar**. Revisa la vista previa y confirma
   para preparar el contexto del proyecto, sincronizar las skills recomendadas,
   registrar trust local y abrir un chat nuevo con el agente elegido.
4. Para trabajar, abre archivos desde el explorador, usa **Terminal +** o
   `Ctrl+Shift+T` para una shell, y ve a **Agentes** para abrir una sesión.
   `Ctrl+P` busca acciones y áreas.

### Primera vez en la TUI

Instala el CLI desde el [instalador reducido](#instalador-reducido), abre una
terminal en el repositorio y prepara el proyecto con vista previa:

```bash
cd /ruta/al/proyecto
chxchx-tech doctor .
chxchx-tech init --dry-run .
chxchx-tech init .
chxchx-tech tui .
```

En **Inicio**, revisa el proyecto y usa **Confiar proyecto** después de revisar
su configuración. `T` abre la terminal del proyecto, `A` prepara y abre agentes,
y `Ctrl+P` busca comandos. Pulsa `q` para salir de la TUI; esto no detiene los
procesos ni cierra la sesión Zellij.

Si al iniciar una sesión Zellij la TUI queda suspendida, vuelve con `Ctrl+O`,
suelta esas teclas y pulsa `D`. El workspace permanece disponible hasta que lo
detengas desde la interfaz o ejecutes `chxchx-tech workspace stop .`.

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

El catálogo contiene 21 skills generales y para Python, TypeScript, React/Next.js,
NestJS, .NET, React Native, PostgreSQL, Prisma, Redis, Docker, diseño UI/UX y
software nativo en C++/Qt. Las guías C++/Qt cubren ingeniería moderna, builds
con CMake, aplicaciones Qt 6 y distribución para Windows, macOS y Linux. La skill
UI/UX se recomienda para proyectos React, Next.js y React Native y forma parte
de sus Tech Packs.
Las recomendaciones no cambian el proyecto. Habilita de forma explícita las
skills que quieres añadir al contexto:

```bash
chxchx-tech skill list
chxchx-tech skill search python
chxchx-tech skill search qt
chxchx-tech skill info python-engineering
chxchx-tech skill recommend .
chxchx-tech skill enable nextjs .
chxchx-tech skill sync . --dry-run
chxchx-tech skill sync .
chxchx-tech pack list
chxchx-tech pack detect .
chxchx-tech pack info qt-cross-platform-desktop
chxchx-tech pack apply-detected . --dry-run
chxchx-tech pack apply-detected .
chxchx-tech skill status .
```

La selección por proyecto se guarda en `.ai/chxchx-skills.toml`; `skill sync`
genera las instrucciones activas en `.ai/SKILLS.md` y añade una referencia
administrada a `AGENTS.md`. Usa `skill disable NOMBRE .` y vuelve a sincronizar
para retirar una skill del contexto.
No instalas cada skill por separado: las 21 guías y 13 Tech Packs vienen con
ChxChx. `skill list` informa el total de guías; `pack list` muestra cada pack,
cuántas skills incluye y sus nombres; `pack info NOMBRE` consulta uno en
particular. Así puedes ver exactamente qué se añadirá antes de aplicarlo.

`pack detect` explica qué stack o lenguaje coincidió. `pack apply-detected`
añade de una vez la unión de los packs compatibles, sin escribir sus nombres;
`--dry-run` permite previsualizar. La selección queda guardada por proyecto,
`skill status` cuenta cuáles están activas y después `skill sync .` carga esa
selección sin volver a nombrar las skills. Son instrucciones Markdown para los
agentes, no paquetes ejecutables ni utilidades instaladas individualmente.

### Mods locales para Claude Code

El catálogo incluye nueve Mods de instrucciones de proyecto: asesor de modelo,
revisión antes de integrar, depuración con evidencia, plan de funcionalidades,
desarrollo nativo C++/Qt, flujo Git, refactorización segura, documentación técnica
y rendimiento basado en mediciones. Se aplican en `.claude/skills/`, no instalan ejecutables,
hooks ni ajustes globales de Claude. Claude Code carga las skills del proyecto al
abrir una sesión nueva; consulta la [documentación oficial de skills](https://code.claude.com/docs/en/skills).

```bash
chxchx-tech mod list --path .
chxchx-tech mod apply --path . --dry-run
chxchx-tech mod apply --path .
chxchx-tech mod apply chxchx-model-advisor --path .
chxchx-tech mod remove chxchx-model-advisor --path . --dry-run
```

`mod apply` instala los Mods recomendados por stack y muestra conflictos antes
de escribir. Los cambios son locales al repositorio, se pueden repetir sin
duplicar archivos y crean backup en `.ai/backups/claude-mods/` al actualizar un
Mod administrado. La preparación rápida de Studio los previsualiza y aplica si
detecta un agente Claude, `.claude/` o `CLAUDE.md`.

El asesor clasifica el trabajo y recomienda Haiku para tareas mecánicas, Sonnet
para trabajo habitual u Opus para problemas difíciles o de alto impacto. Indica
el comando oficial, como `/model sonnet`; el usuario confirma el cambio dentro
de Claude Code antes de continuar. No cambia el modelo silenciosamente.

## Requisitos

Obligatorios:

- Python `>=3.11`;
- Git;
- `uv`;
- una terminal.

Para usar todas las funciones del workspace, instala también según necesidad:

- Zellij para sesiones persistentes;
- ChxChx Studio para editar (Sublime Text se conserva como opción externa);
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

### Instalar ChxChx Studio sin clonar el repositorio

Cuando se publique el preview, podrás instalar la aplicación de escritorio y su
bridge CLI desde sus paquetes. El instalador detecta el sistema y la arquitectura,
descarga solo esos archivos, comprueba sus SHA-256 y configura una carpeta
aislada por versión. No descarga ni clona el código fuente del repositorio.

| Sistema | Paquete disponible | Instalación |
| --- | --- | --- |
| Ubuntu 22.04 o posterior, x86_64 | TGZ con runtime Qt | Bash; `sudo` instala bibliotecas gráficas del sistema |
| Fedora x86_64 | TGZ con Qt y sus plugins gráficos | Bash; `sudo` instala bibliotecas gráficas del sistema |
| macOS Apple Silicon e Intel | DMG con app `.app` | Bash; primera apertura puede requerir confirmar la app sin firma |
| Windows 10/11 x86_64 | `Setup.exe` más paquete Qt y bridge CLI | Instalador gráfico; instala solo en el perfil del usuario |

Estos comandos instalan el **preview de desarrollo** más reciente de la rama
`dev`, no una versión estable. CI construye los instaladores después de validar
los builds. La publicación del preview permanece aplazada; los comandos estarán
disponibles cuando se publique el tag
[studio-dev-preview](https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/studio-dev-preview).

#### Linux y macOS

Primero puedes descargar y verificar el paquete sin instalarlo. El modo
`--dry-run` descarga a una carpeta temporal, valida los checksums y no modifica
el sistema ni tu instalación de Studio.

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.sh | bash -s -- --dry-run
```

Cuando quieras instalar:

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.sh | bash
```

El mismo comando vuelve a instalar la versión publicada más reciente cuando
Studio ya está instalado. También puedes indicar `--update` para exigir una
instalación previa; combínalo con `--dry-run` para revisar la descarga antes de
aplicarla:

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.sh | bash -s -- --dry-run --update
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.sh | bash -s -- --update
```

En Ubuntu y Fedora el paquete incluye Qt y sus plugins; el instalador solicita
`sudo` para instalar las bibliotecas gráficas del sistema que Qt necesita para
abrir una ventana. Studio queda en `~/.local/opt/chxchx-studio/`
y el comando `chxchx-studio-dev` en `~/.local/bin/`. Abre un proyecto así:

```bash
chxchx-studio-dev /ruta/a/tu/proyecto
```

En macOS puedes abrirlo también desde `~/Applications`. Al ser un preview sin
firma de desarrollador, macOS puede bloquear la primera apertura; desde Finder,
usa **clic derecho → Abrir** y confirma una sola vez. El preview no requiere
Homebrew ni clonar el repositorio.

En Linux/macOS, si el comando no aparece en una terminal nueva, agrega
`~/.local/bin` a `PATH`; en macOS también puedes abrir la app desde Finder.

#### Windows 10/11

Descarga y ejecuta el instalador gráfico. No necesita permisos de administrador;
descarga Studio, valida los checksums e instala el bridge CLI en el perfil actual.

```powershell
$installer = Join-Path $env:TEMP 'chxchx-studio-setup.exe'
Invoke-WebRequest 'https://github.com/chxchx-dev/chxchx-tech-lead/releases/download/studio-dev-preview/chxchx-studio-windows-x86_64-setup.exe' -OutFile $installer
Start-Process -FilePath $installer -Wait
Remove-Item $installer
```

Si quieres revisar antes de instalar, también puedes usar el script PowerShell
con `-DryRun`:

```powershell
$installer = Join-Path $env:TEMP 'install-chxchx-studio.ps1'
Invoke-WebRequest 'https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.ps1' -OutFile $installer
powershell -ExecutionPolicy Bypass -File $installer -DryRun
powershell -ExecutionPolicy Bypass -File $installer
Remove-Item $installer
```

Para actualizar una instalación existente, usa el mismo archivo con `-Update`;
`-DryRun` revisa primero las descargas y posibles conflictos:

```powershell
$installer = Join-Path $env:TEMP 'install-chxchx-studio.ps1'
Invoke-WebRequest 'https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.ps1' -OutFile $installer
powershell -ExecutionPolicy Bypass -File $installer -DryRun -Update
powershell -ExecutionPolicy Bypass -File $installer -Update
Remove-Item $installer
```

Puedes volver a ejecutar `Setup.exe` para actualizar. Studio queda instalado en
`%LOCALAPPDATA%\Programs\ChxChxStudio\`; también aparece un acceso **ChxChx
Studio Dev** en el menú Inicio. El bridge CLI queda en un entorno virtual propio
de esa versión y no reemplaza otro `chxchx-tech`.

#### Actualizar y desinstalar

Vuelve a ejecutar el instalador o usa `--update` (PowerShell: `-Update`) para
obtener el último commit publicado. Studio también ofrece **Ayuda → Buscar
actualizaciones…**: muestra la versión disponible y pide confirmación antes de
descargarla. Tras instalarla, cierra y vuelve a abrir Studio. Cada commit queda
en una carpeta propia y la versión anterior se conserva como rollback; el
launcher y el acceso de macOS apuntan a la nueva versión. En Linux/macOS, quita `~/.local/bin/chxchx-studio-dev`
y las carpetas `~/.local/opt/chxchx-studio/dev-*`. En macOS, quita además el
enlace `~/Applications/ChxChx Studio Dev.app` y las apps
`~/Applications/ChxChx Studio Dev-*.app`. En Windows elimina la carpeta
`%LOCALAPPDATA%\Programs\ChxChxStudio\` y el acceso del menú Inicio. Las
bibliotecas de Ubuntu/Fedora y `uv` se instalan en el usuario/sistema y se
administran aparte; no las elimines si las usan otras herramientas.

Las rutas ya existentes sin el marcador `.chxchx-managed` no se sobrescriben.
Las descargas se comprueban contra el manifiesto del release antes de instalar.

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

# 8. Abrir Studio y trabajar con el editor o el agente
chxchx-tech studio open .
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
La paleta de la TUI abre el mismo ChxChx Studio instalado. Si un proyecto aún
configura `editor = "sublime"`, ese editor externo sigue disponible desde la CLI
con `chxchx-tech editor open .`; los proyectos nuevos usan Studio.

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
editor = "studio"
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
# o para agentes configurados compatibles (Codex, Claude Code y OpenCode):
chxchx-tech agent start --all --new-chat --path .
chxchx-tech workspace attach .
```

`--new-chat` está disponible para Codex, Claude Code y OpenCode. Al terminar una sesión, ChxChx captura solicitudes explícitas de memoria de Codex/Claude y, para OpenCode, exporta las sesiones del proyecto con su CLI oficial y `--sanitize`. Comprueba que cada sesión pertenezca a la raíz del proyecto, omite preguntas de recuperación y mensajes que parezcan contener secretos, y guarda las solicitudes en `.ai/memory/PROJECT_MEMORY.md` sin duplicarlas. Al abrir un chat nuevo, recupera la memoria del mismo proyecto. En Studio puedes conectar Basic Memory y Serena desde `Configuración → Configurar MCP · OpenCode`; también está disponible `chxchx-tech integrate . --client opencode`. Esto configura MCP en el proyecto con la CLI oficial de OpenCode, sin tocar su configuración global. `.ai/` es local a cada checkout y está excluido de Git.

Puedes incorporar agentes al proyecto sin editar el TOML a mano:

```bash
chxchx-tech agent add opencode --path . --dry-run
chxchx-tech agent add opencode --path .
# Opción guiada para usar OpenCode con modelos de Ollama:
chxchx-tech agent add ollama-opencode --path .
```

La operación es idempotente, conserva el resto de la configuración y crea un
backup antes de escribir. En Studio, abre `Agentes → Añadir agente`, elige
OpenCode o OpenCode con Ollama local, revisa la vista previa y confirma. Luego
usa `Configuración → Configurar MCP · OpenCode` para conectar Basic Memory y Serena
al proyecto. El agente aparece en el selector normal y mantiene el preflight de
confianza y RAM de Studio. Como cambió el hash de configuración, vuelve a dar trust al
proyecto antes del primer inicio. Para la ruta guiada local, instala Ollama
0.15 o posterior; al iniciar `ollama launch opencode`, Ollama guía la selección del modelo. Los
requisitos de memoria dependen del modelo y su contexto elegido. Consulta la
[guía oficial de Ollama Launch](https://ollama.com/blog/launch) y la
[documentación de OpenCode CLI](https://dev.opencode.ai/docs/cli/).

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
chxchx-tech projects prepare [PATH]     prepara skills/packs y trust local
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
chxchx-tech studio open [PATH]         abre el proyecto en ChxChx Studio
chxchx-tech editor open [PATH]         abre el editor configurado (Studio por defecto)
chxchx-tech editor setup [PATH]        prepara un proyecto Sublime opcional
chxchx-tech agent start ID --path .    inicia un agente configurado
chxchx-tech agent add opencode --path . añade OpenCode (acepta --dry-run)
chxchx-tech agent add ollama-opencode --path . añade OpenCode con Ollama local
chxchx-tech agent start ID --new-chat --path . inicia chat nuevo con contexto (Codex/Claude/OpenCode)
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

## Usar la TUI

La TUI es el dashboard interactivo de Textual. Desde la raíz del proyecto abre:

```bash
chxchx-tech tui .
```

### Recorrido de trabajo

En **Inicio** revisa el estado, confianza y recursos del proyecto. Desde ahí:

1. Pulsa **Confiar proyecto** después de revisar `.ai/chxchx-tech.toml`.
2. Pulsa **T** para abrir la terminal del proyecto o **A** para preparar y abrir agentes.
3. Usa **Trabajo** para operar procesos, agentes y terminales.
4. Usa **Proyectos** para cambiar a otro proyecto registrado.
5. Abre **Skills** para revisar recomendaciones, aplicar Tech Packs y sincronizar el contexto.
6. En **Más** están Recursos, Handoff, Notas, Chats, Errores, Configuración y Guía; **CI/CD** tiene sus vistas propias.

La TUI conserva la selección del proyecto activo. Cuando cambia la configuración de workspace, vuelve a revisar el trust antes de iniciar agentes o procesos. El preflight de RAM puede pedir confirmación si se superan los límites establecidos; no detiene procesos automáticamente.

El panel Recursos muestra RAM, swap, CPU y consumo estimado de los procesos
gestionados. Ajusta los avisos en `[workspace.resources]` de
`.ai/chxchx-tech.toml` con `warn_memory_percent`, `critical_memory_percent`,
`warn_swap_percent` y `max_agents`; la estimación usa los PID principales y no
incluye procesos hijos ni agentes dentro de panes Zellij.

### Teclas y terminales

```text
T        abrir la terminal del proyecto
A        preparar y abrir agentes
Ctrl+P   buscar un comando
1–4      Inicio, Trabajo, Proyectos y Más
r        actualizar la vista
q        salir de la TUI; no detiene el workspace
```

Al entrar a Zellij, la TUI suspende su control de la terminal. Para regresar,
pulsa `Ctrl+O`, suelta las teclas y pulsa `D`. La sesión sigue abierta. Para
detener los procesos administrados, usa **Detener procesos** o ejecuta:

```bash
chxchx-tech workspace stop .
```

La TUI ejecuta operaciones largas en segundo plano. Para obtener ayuda sobre
una acción desde la terminal, consulta `chxchx-tech --help` o
`chxchx-tech NOMBRE --help`.

## CI opcional para tus proyectos

ChxChx puede preparar un workflow de GitHub Actions para proyectos Python,
Node.js y CMake/C++/Qt. Primero revisa la propuesta; solo se escribe el archivo
cuando confirmas la instalación:

```bash
chxchx-tech ci setup . --dry-run
chxchx-tech ci setup .
chxchx-tech ci status
```

La plantilla ejecuta comprobaciones en `push`, `pull_request` y `workflow_dispatch`,
con permisos `contents: read`. Las acciones externas quedan fijadas a commits;
no incorpora secretos ni inicia despliegues. Para activarla en GitHub, sube
`.github/workflows/chxchx-ci.yml` al repositorio. El workflow debe existir en la
rama por defecto antes de pedir una ejecución manual.

Con GitHub CLI (`gh`) instalada y autenticada puedes seguir ejecuciones, lanzar
una ejecución manual y consultar logs fallidos:

```bash
chxchx-tech ci status
chxchx-tech ci run --ref main
chxchx-tech ci logs 123456789
```

La TUI y Studio muestran estado, ejecuciones y logs. Escribir, retirar o
ejecutar CI manualmente pide confirmación; las consultas son de solo lectura.
Si el workflow ya existe sin el marcador de ChxChx, se conserva.
Si alguien modifica el archivo administrado, ChxChx detiene la actualización o
retirada hasta que se revise. Al actualizar o retirar un workflow íntegro, se
guarda un backup fuera del repositorio. Para retirarlo, usa `ci remove --dry-run`
y luego `ci remove`.

### CD opcional a tu servidor o VPS

Para proyectos que ya usan Docker Compose, ChxChx puede preparar un workflow
GitHub Actions que despliega por SSH a un servidor propio o VPS. El despliegue
es manual por defecto; activar una rama para despliegue automático es opcional.
ChxChx no despliega este repositorio ni almacena credenciales del servidor.

```bash
chxchx-tech deploy setup . --dry-run
chxchx-tech deploy setup .
chxchx-tech deploy status .
chxchx-tech deploy run . --dry-run
chxchx-tech deploy run . --confirm
chxchx-tech deploy logs 123456789 --path .
chxchx-tech deploy remove . --dry-run
chxchx-tech deploy remove .
```

El asistente inicial admite un único `compose.yaml`, `compose.yml`,
`docker-compose.yaml` o `docker-compose.yml` en la raíz. En Studio, abre
**CD VPS**; en la TUI usa la pestaña **Despliegue VPS**. Ambas interfaces
previsualizan setup y retirada, y piden confirmación antes de despachar un
despliegue remoto. Para usar el modo automático:

```bash
chxchx-tech deploy setup . --auto-deploy-on main --dry-run
chxchx-tech deploy setup . --auto-deploy-on main
```

#### Preparar GitHub y el VPS

1. Instala Docker Engine y el plugin Docker Compose en el VPS. Crea un usuario
   dedicado sin iniciar sesión como `root`, clona allí este mismo repositorio y
   confirma que `origin` apunta al repositorio GitHub correcto. El árbol
   versionado debe estar limpio. El usuario necesita permisos para actualizar
   el checkout y ejecutar Compose; pertenecer al grupo `docker` puede equivaler
   a tener acceso root, así que considera Docker rootless.
2. Mantén la configuración de producción en el servidor. Por ejemplo, coloca
   `.env` dentro de `VPS_DEPLOY_PATH`; Git y este workflow la dejan intacta. Si
   el repositorio es privado, instala en el VPS una llave de lectura para que
   `git fetch` pueda obtener el código. Esa llave es distinta de la llave SSH
   que permite a GitHub Actions conectarse al VPS.
3. En GitHub crea el Environment `production` y agrega estas **variables**:
   `VPS_SSH_HOST` (nombre DNS o dirección IPv4), `VPS_SSH_USER` (usuario dedicado) y
   `VPS_DEPLOY_PATH` (ruta absoluta del clon). `VPS_SSH_PORT` es opcional y
   usa el puerto 22 por defecto.
4. En los secretos del Environment configura `VPS_SSH_PRIVATE_KEY`, la llave
   privada cuya llave pública está autorizada para el usuario del VPS, y
   `VPS_SSH_KNOWN_HOSTS`, con la entrada conocida del host. Obtén y verifica
   la huella del host por un canal confiable antes de guardarla; la conexión
   exige validación estricta y no acepta claves aprendidas automáticamente.
5. Revisa y sube `.github/workflows/chxchx-deploy.yml`. Para el primer despacho
   manual, GitHub requiere que el workflow exista en la rama por defecto.
   Después ejecuta `deploy run . --dry-run`, revisa la referencia y confirma
   con `deploy run . --confirm`. Puedes exigir aprobación adicional desde las
   reglas del Environment `production`.

El workflow usa permisos GitHub mínimos (`contents: read`), serializa los
despliegues de producción, actualiza el VPS al commit elegido y ejecuta
`docker compose up -d --build`. Comprueba que los servicios permanezcan en
`running` y que sus healthchecks no estén fallando. Si el inicio o la
comprobación falla, intenta volver al commit previo y reconstruirlo; el log de
la ejecución informa si también falló ese rollback. Configura `healthcheck` en
Compose para comprobar que la aplicación está lista. Esta reversión no puede
deshacer cambios destructivos de esquema o datos; prepara las migraciones de
base de datos para que sean compatibles con la versión previa. Los servicios
del workflow inicial deben ser procesos de larga duración; proyectos con tareas
Compose de una sola ejecución necesitan ajustar el workflow generado.

El setup detecta colisiones y protege los workflows ajenos o editados. Para
actualizar o retirar un workflow administrado, genera backup fuera del checkout.
La configuración de CD es independiente de la CI y puedes dejarla desactivada.

## Usar ChxChx Studio

Studio es la aplicación de escritorio para editar archivos y operar el workspace
en una ventana. La instalación y actualización están explicadas en
[Instalar ChxChx Studio sin clonar el repositorio](#instalar-chxchx-studio-sin-clonar-el-repositorio).

### Abrir un proyecto

Pasa la ruta del proyecto al launcher para abrirlo directamente:

```bash
chxchx-studio-dev /ruta/al/proyecto
```

En Windows PowerShell:

```powershell
& "$env:LOCALAPPDATA\Programs\ChxChxStudio\chxchx-studio-dev.cmd" "C:\ruta\al\proyecto"
```

También puedes abrir la app desde Finder o el menú Inicio. Si la abres sin una
ruta, Studio usa el directorio de trabajo actual; **Proyectos** permite cambiar
entre proyectos ya registrados. Para abrir otro repositorio directamente, usa
el launcher con su ruta como en los ejemplos anteriores. Si trabajas en este
checkout, consulta [Compilar Studio desde el checkout](#compilar-studio-desde-el-checkout).

### Primer recorrido

1. Ve a **Inicio** y pulsa **Preparar y entrar**.
2. Revisa la vista previa. Al confirmar, Studio prepara el contexto local,
   sincroniza skills recomendadas y Tech Packs detectados, confirma trust y
   abre un chat nuevo con el agente seleccionado.
3. Abre un archivo desde el explorador para editarlo. Usa **Terminal +** o
   `Ctrl+Shift+T` para abrir la shell del proyecto.
4. En **Agentes**, selecciona una CLI configurada o añade OpenCode desde
   **Añadir agente**. Las sesiones nuevas pueden incluir el contexto y memoria
   del proyecto.

Si todavía no tienes un agente configurado, puedes preparar el proyecto desde
**Configuración** y añadir OpenCode desde **Agentes**. Codex y Claude Code deben
estar instalados por separado y configurados como agentes del proyecto.

### Navegar y trabajar

| Área | Para qué sirve |
| --- | --- |
| Explorador | Abrir archivos y desplegar carpetas |
| Inicio | Preparar el proyecto y acceder a acciones frecuentes |
| Proyecto | Consultar trust y estado; iniciar, suspender o detener el workspace |
| Proyectos | Cambiar entre proyectos registrados |
| Agentes | Iniciar una CLI, abrir un chat nuevo o conectar OpenCode |
| Procesos | Iniciar y detener comandos configurados |
| Skills y Tech Packs | Revisar recomendaciones, aplicar instrucciones y sincronizar el contexto |
| Handoff y Memoria | Guardar el siguiente paso y consultar notas locales |
| Chats y Errores | Buscar transcripciones disponibles y errores recientes |
| CI/CD y Recursos | Consultar automatización y uso de recursos |

Atajos útiles: `Ctrl+P` abre la paleta, `Ctrl+Shift+F` busca archivos o contenido
del proyecto, `Ctrl+Shift+O` muestra declaraciones del archivo actual,
`Ctrl+Shift+T` abre una terminal y `Shift+PageUp/PageDown` recorre su historial.

Studio vuelve a comprobar trust y uso de RAM antes de iniciar agentes. Las
acciones que modifican archivos o configuración muestran una vista previa y
piden confirmación. El menú **Ayuda** incluye una guía rápida y la búsqueda
manual de actualizaciones.

### Memoria entre sesiones

Las solicitudes explícitas de memoria se guardan en
`.ai/memory/PROJECT_MEMORY.md`, dentro del proyecto. Codex, Claude y OpenCode
pueden recuperar esas notas al abrir un chat nuevo. La carpeta `.ai/` es local
al checkout y está excluida de Git; copia o sincroniza esa carpeta si necesitas
llevar la memoria a otro clon. Studio no guarda transcripciones completas como
memoria.

### Compilar Studio desde el checkout

Para desarrollar la aplicación en Linux, macOS o WSL, instala CMake 3.21+ y
Qt 6.4+ con Core, Widgets y Core5Compat. En Fedora están disponibles
`qt6-qtbase-devel` y `qt6-qt5compat-devel`; en Ubuntu instala los paquetes de
desarrollo Qt equivalentes.

Este script configura, compila y abre Studio con el checkout como proyecto:

```bash
./scripts/dev-studio.sh
```

Pasa otra ruta para abrirla, o cambia el directorio de build con
`CHXCHX_BUILD_DIR`:

```bash
./scripts/dev-studio.sh /ruta/al/proyecto
CHXCHX_BUILD_DIR=/tmp/chxchx-build ./scripts/dev-studio.sh
```

El flujo manual también está disponible:

```bash
cmake -S native -B build/native -DCHXCHX_BUILD_TESTS=ON
cmake --build build/native
ctest --test-dir build/native --output-on-failure
./build/native/chxchx-studio /ruta/al/proyecto
```

En Windows, ejecuta `build/native/chxchx-studio.exe` con la ruta del proyecto
como argumento. Studio usa Qt Widgets, Scintilla/Lexilla para editar y libvterm
para las sesiones interactivas; CMake descarga dependencias fijadas la primera
vez que configura y necesita acceso a GitHub. Consulta
[native/THIRD_PARTY.md](native/THIRD_PARTY.md) para versiones y licencias.

CPack crea un ZIP de Windows para el actualizador interno, DMG para macOS y TGZ
para Linux. CI empaqueta además un `Setup.exe` que reutiliza el instalador
PowerShell y valida los runtimes Qt de cada paquete antes de cargar los
artefactos. El paquete Fedora lleva Qt y sus plugins dentro del TGZ; el
instalador añade las bibliotecas gráficas del sistema. La edición aún no incluye
LSP y falta QA manual de los paquetes en cada plataforma.

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
