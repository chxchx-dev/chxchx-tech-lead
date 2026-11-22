# Manual técnico de ChxChx

Detalle de capacidades, seguridad y decisiones del producto. Para una visión general, empieza por el [README](README.md).
La versión es `0.6.3` (la línea 0.6 es el producto Rust); reemplaza a la instalación 0.5.x al actualizar.

## Capacidades compartidas actuales

`project-overview` lee una ruta de proyecto y entrega el mismo DTO a CLI, TUI y
Studio. Incluye marcadores, stacks, lenguajes, gestores, infraestructura y perfil
resuelto; lee si el TOML de `.ai/chxchx-tech.toml` se puede parsear, su versión,
compatibilidad estructural inicial y trust local. Comprueba límites de recursos,
comandos declarados y que sus directorios de trabajo permanezcan dentro del
proyecto; no ejecuta comandos. El detector omite carpetas de dependencias/build como
el legado y carga perfiles incluidos más los locales en
`$CHXCHX_TECH_HOME/profiles/` (o `~/.chxchx-tech-lead/profiles/`). No valida aún
el esquema ChxChx completo, inicia procesos ni modifica archivos.
Su JSON es `chxchx.project-overview` v1 y está en `schemas/`.

`init [PATH]` previsualiza la inicialización completa; `--minimal` prepara solo
configuración, memoria y registro local. `--apply` aplica el plan mostrado.
Ambos modos comparten backups, preservan configuración existente, migran TOML
v1→v2 cuando corresponde y actualizan el registro local. Si Basic Memory está
instalado, `--apply` asegura que `.ai/memory` esté registrado allí; la vista
previa no ejecuta su CLI. Si el registro falla, init conserva los cambios
locales aplicados y muestra un aviso. TUI y Studio aplican en segundo plano.
El init completo también crea las plantillas administradas y el ejemplo MCP de
OpenCode; configura clientes MCP solo con `integrate --apply`.

`integrate [PATH] --client all|claude|codex|opencode` previsualiza la
configuración MCP compartida por CLI, TUI y Studio. Solo `--apply` invoca los
CLIs oficiales y asegura el proyecto Basic Memory. `doctor [PATH]` inspecciona
herramientas y configuraciones sin ejecutar procesos declarados.

`projects list` muestra alias, nombre, perfil, stacks, disponibilidad y ruta;
`projects current` resuelve el último proyecto disponible. Ambas lecturas usan
el registro local y están compartidas por CLI, TUI (`p`) y Studio.
`projects sync` muestra una vista previa; solo `projects sync --apply` escribe
los bloques administrados en `AGENTS.md` y `CLAUDE.md`. Revisa snapshots del
registro y archivos, respalda cada proyecto modificado e intenta rollback si una
escritura falla. Omite proyectos ausentes o inseguros con avisos. CLI, TUI (`s`,
luego `y`) y Studio usan el mismo caso de uso. `projects switch` espera al runtime
de workspace/agentes.

`jobs run --project ALIAS --provider claude|codex|opencode "mensaje"` habla con
el agente de un proyecto registrado y de confianza sin abrir Studio ([ADR
0017](.ai/adr/0017-local-node-and-remote-access.md)). Acepta `--access` (por
defecto `workspace-write`), `--network`, `--model`, `--effort`, `--session` y
`--key`. Este último es una clave de idempotencia: repetir el comando con la
misma clave devuelve el trabajo existente en lugar de crear otro. La respuesta se
muestra en vivo y la conversación queda en `.ai/chat-state/`, igual que en
Studio, que la retoma al abrirlo. Sin nadie que apruebe, cada permiso o pregunta
del agente se rechaza y se anota en el trabajo; las aprobaciones remotas llegan
con el nodo. Los trabajos se guardan en `$CHXCHX_TECH_HOME/jobs/`. Si el comando
se corta, el trabajo queda `interrupted` con su historial, y nada lo reanuda solo.
`jobs list [--json]` y `jobs show ID [--events] [--json]` los consultan sin
modificarlos. Si `jobs run` se corta, el agente puede seguir vivo hasta terminar
su turno. Claude corre en su propio grupo de procesos, así que `Ctrl+C` en la
terminal no lo detiene; Codex y OpenCode, a través del servicio, sí lo reciben.
Cancelar de forma limpia un trabajo en curso llega con el nodo (H2).

### Nodo local (feature `node`, en desarrollo)

`chxchx-tech node start [--port 47821]` levanta el nodo en primer plano. Solo se
compila con `--features node` y todavía no entra en los instaladores. El nodo
administra los trabajos de este usuario. Mientras corre, `jobs run` lo indica y
pide usar su API. Escucha **solo en `127.0.0.1`**: rechaza toda petición cuyo
`Host` u `Origin` no sea `127.0.0.1` o `localhost` en su puerto, lo que protege
contra páginas web que intenten llamarlo y contra el DNS rebinding. Todas las
rutas, salvo `GET /api/v1/health`, piden el token de
`$CHXCHX_TECH_HOME/node/token` (`Authorization: Bearer …`), un archivo `0600`
dentro de una carpeta `0700` que se crea una vez y se reutiliza. Rutas:
`GET /api/v1/node`, `GET|POST /api/v1/jobs` (con cabecera `Idempotency-Key`
opcional), `GET /api/v1/jobs/{id}`, `GET /api/v1/jobs/{id}/events?since=N`,
`POST /api/v1/jobs/{id}/cancel`, `POST /api/v1/jobs/{id}/answers` y
`POST /api/v1/node/stop`. `GET /api/v1/events` es un WebSocket (subprotocolo
`chxchx.v1`; el token puede ir como subprotocolo `bearer.<token>`) que envía cada
`JobEvent` en vivo. Con `?job=ID&since=N` manda antes lo que ese trabajo tuvo
después de `N`, para que una reconexión reciba solo lo que le faltó. Cuando el
agente pide permiso o pregunta algo, el trabajo espera la respuesta en
`…/answers` (`{"request_id", "behavior": "allow"|"deny", "scope"?}` o
`{"request_id", "answers": {pregunta: respuesta}}`) hasta 10 minutos. Si nadie
responde, la petición se rechaza y queda anotada como evento `answered`. El nodo
escribe `node/node.log` (rota a 1 MiB) sin tokens ni textos de respuestas. Los errores son JSON
`{code, message}` con el código HTTP correspondiente, y el cuerpo de una petición
está limitado a unos 320 KiB. `node status` y `node stop` usan el mismo token.
Al detenerse (Ctrl+C o `node stop`), el nodo cancela lo que esté en curso, espera
hasta 10 s y borra `node.json`.

**Dispositivos vinculados (H3).** Un celular u otro equipo no usa el token local:
se vincula en la PC. `chxchx-tech node pair [--role reader|operator|admin]` crea un
código de un solo uso (`XXXXX-XXXXX`, 5 min). El dispositivo lo envía a
`POST /api/v1/pair` con un nombre (`{"code","name"}`) y recibe **su propio token,
que se muestra una sola vez**. El nodo solo guarda su hash SHA-256 (también el del
código), así que `devices.json` no sirve para entrar. Cinco códigos incorrectos
cierran la vinculación diez minutos y cancelan los códigos pendientes. Roles:
`reader` mira proyectos, trabajos y eventos; `operator` además lanza y cancela
trabajos y responde a permisos; `admin` además gestiona dispositivos
(`node devices`, `node revoke ID`) y lee `GET /api/v1/audit`. Detener el nodo y
crear códigos de vinculación solo se puede hacer con el token local, en la PC.
Revocar corta el token en la siguiente petición y cierra los eventos en vivo que el
dispositivo tuviera abiertos. Los trabajos que pide un dispositivo se recortan
**antes de guardarlos**: el acceso nunca pasa de `workspace-write`, aunque pida
`full-access`, y la red solo se permite a un `admin`. Lo que se lista, lo que
corre y lo que se audita es lo efectivo. `GET /api/v1/projects` lista solo los
proyectos de confianza, sin rutas. `node audit` muestra quién hizo qué (vinculaciones,
revocaciones, trabajos, cancelaciones, respuestas y cambios de acceso) y no
necesita el nodo en marcha; nunca guarda prompts, textos de respuesta ni tokens. El nodo
atiende como mucho 128 peticiones a la vez, 16 flujos de eventos y 50 trabajos en
cola. Las claves de idempotencia se ligan a quien las envía (la clave completa, con
su dueño, no pasa de 128 caracteres).

**Tu identidad.** El nombre que se dibuja en letras de bloques, el lema, la imagen y el tema son tuyos.
En Studio: Ajustes → Identidad (vista previa en vivo; «Elegir imagen…» abre el selector de archivos). En la
línea de comandos: `chxchx-tech identity show`; `identity colors` lista los degradados y `identity set --colors sunset` pone uno; `identity set --name "ana dev" --tagline "mi taller" --avatar
foto.png --theme forest --accent "#f472b6" --web-theme studio` (añade `--dry-run` para ver cómo quedaría);
`identity clear-avatar`; `identity reset`; `identity themes`. El nombre admite hasta 12 caracteres (letras sin
tilde, números, espacio, `-` y `.`). Lo que cambies se ve en Studio y, al recargar, en la app web del teléfono;
nada se borra (lo anterior queda como `.bak` en la carpeta `identity` de ChxChx). `--web-theme auto` hace que la
app web siga al dispositivo (claro u oscuro) y `studio` que use tu tema de Studio; el color de resalte se aplica
en los dos casos.

**Manager (H6).** `chxchx-tech manager status` dice qué tiene la máquina y `manager plan`
qué falta; `manager apply [--dry-run]` hace solo lo que es de ChxChx. `manager watch` mantiene vivos
el nodo y el servicio de agentes (si algo se cae 3 veces en una hora, se detiene y lo explica) y
`manager recover [--dry-run]` repara copias viejas o rotas del servicio. `manager update` busca una
versión firmada más nueva y `--apply` la instala (solo Linux; espera a que no haya trabajos y guarda
la anterior en `.update-backup`). Para publicar actualizaciones el mantenedor ejecuta una vez
`manager keygen --out ARCHIVO`, guarda la clave privada como secreto `RELEASE_SIGNING_KEY` en GitHub
y sube la pública a `packaging/release-public-key.txt`.

**Studio y servicio de usuario (H5).**
En Ajustes → *Nodo local* ves si el nodo corre, lo inicias o detienes, activas «Usar el nodo
para los turnos del chat» (cerrar Studio no corta el turno; al reabrir lo recupera) y gestionas
dispositivos: *Vincular un dispositivo* muestra un código de un solo uso y *Desvincular* pide
confirmación. Para que el nodo arranque al iniciar sesión:
`chxchx-tech node service install` (vista previa), `... install --apply`, `... status`,
`... uninstall --apply`. Es un servicio de tu usuario (nunca root); en Linux, para que corra sin
sesión abierta, ejecuta tú `loginctl enable-linger`. macOS y Windows aún sin verificar.

**Acceso desde tus dispositivos por Tailscale (H3).**
`chxchx-tech node remote status|enable|disable` configura `tailscale serve` para que
el nodo, que solo escucha en `127.0.0.1`, responda también al nombre de tu máquina
en la tailnet (`https://máquina.tailnet.ts.net/`). Como el resto del producto,
`enable` y `disable` muestran una vista previa con el comando exacto y solo
`--apply` lo ejecuta; `disable` es la vuelta atrás. Solo usa `tailscale serve` y
`tailscale status`: nunca `funnel` (no sale a Internet), no toca el firewall, no
cambia un puerto HTTPS que ya sirva otra cosa y rechaza uno con Funnel activo.
Con el puerto fijo `47821` (por defecto de `node start`) el acceso sigue
funcionando entre reinicios; si el nodo se inicia con `--port 0`, repite
`enable --apply`. La primera vez Tailscale tarda hasta ~1 min en emitir el
certificado HTTPS. El nodo solo acepta ese nombre si `node/remote.json` lo dice y
solo para nombres `*.ts.net`. El token local no vale por la tailnet: cada
dispositivo se vincula con su código. Ver [.ai/NODE-SECURITY.md](.ai/NODE-SECURITY.md)
para los riesgos y lo que queda abierto.

**App web (PWA, H4).** El nodo sirve en `/` una app web (Dioxus compilado a WebAssembly,
`apps/remote`) para el teléfono o cualquier navegador: ver proyectos de confianza, hablar con
Claude, Codex u OpenCode con la respuesta en vivo, aprobar o rechazar sus permisos, responder sus
preguntas, cancelar trabajos y, siendo administrador, desvincular dispositivos. Se instala desde el
menú del navegador («Instalar app» en Android, «Añadir a pantalla de inicio» en iPhone), arranca
sin red desde su service worker y, si cae la conexión, se reconecta y recupera lo que pasó. Para
incluirla en el nodo: `scripts/build-remote.sh && cargo build --features node` (necesita
`rustup target add wasm32-unknown-unknown` y `cargo install wasm-bindgen-cli` en la versión de
`Cargo.lock`); sin ese paso el nodo funciona igual y su portada lo indica. La primera vez se
vincula con el código de `chxchx-tech node pair` o, con el acceso remoto activado, escaneando el QR
que imprime (la dirección lleva el código tras la `#`, que no llega a ningún servidor). El acceso
predeterminado es solo leer; «leer y editar» se elige en el chat o en Ajustes. Los avisos son
locales (mientras la página viva): no hay notificaciones push. Pruebas de navegador en
`apps/remote/e2e`.

Skills, Tech Packs y Mods están portados (CLI y Studio). El catálogo del legado
(40 skills, 22 packs, 31 mods) se empaqueta en el binario al compilar leyendo
`src/chxchx_tech_lead/`, sin copiarlo. Todo cambio se previsualiza y solo se aplica
con `--apply` (botón Confirmar en Studio):

```powershell
chxchx-tech skills list .                      # ✓ habilitada, ★ recomendada
chxchx-tech skills enable architecture testing --path . --apply
chxchx-tech packs apply python-backend --path . --apply   # habilita todas sus skills
chxchx-tech mods apply --path .                # sin nombres: los recomendados
chxchx-tech mods remove chxchx-review-before-merge --path . --apply
```

Habilitar skills escribe `.ai/chxchx-skills.toml`, regenera `.ai/SKILLS.md` y el
bloque administrado de `AGENTS.md` (backup previo; el texto manual se conserva).
Los Mods se instalan como skills de agente en `.claude/skills/` y, si el proyecto
usa Codex, también en `.agents/skills/`. Un archivo con el mismo nombre que no
administra ChxChx nunca se sobrescribe ni se retira, y lo reemplazado queda en
`.ai/backups/claude-mods/`. La TUI aún no ofrece esta capacidad.

## Interfaz del Studio

- **Barra de menú** (Archivo, Editar, Ver, Ir, Proyecto, Ayuda): cada comando del
  Studio tiene su entrada, con el atajo a la derecha. Un menú abierto cambia al pasar
  el ratón por otro; `Esc` o un clic fuera lo cierra. Los comandos que no aplican
  (Guardar sin cambios, Deshacer sin archivo) aparecen atenuados, no ausentes.
- **Barra de herramientas**: Inicio, Guardar, Deshacer, Rehacer, buscar en el archivo,
  buscar en el proyecto, Preparar, Catálogo, Conectar y Salud. Se resalta la sección
  actual y lo que no aplica se desactiva; cada botón tiene tooltip con su atajo.
- **Barra de actividad**: dos grupos con una marca cada uno: la barra lateral izquierda
  (Explorador, Buscar o Espacio de trabajo) y el panel inferior (Terminal o Actividad).
  Pulsar el elegido otra vez oculta ese panel. No hay panel derecho: todo el ancho es
  del editor. El **Espacio de trabajo** agrupa las secciones (Proyecto, Agentes y
  herramientas, Salud, Continuidad) bajo una tarjeta con el estado del proyecto; también
  se abren desde el menú Ir y la barra de herramientas. La vista elegida se recuerda.
- **Buscar en el proyecto** (`Ctrl+Shift+F`): por contenido o por nombre, máximo 200
  coincidencias, omite carpetas de dependencias, enlaces, binarios y `.env*`. Un clic
  abre el archivo en la línea y selecciona la coincidencia.
- Atajos: `Ctrl+B` barra lateral, `Ctrl+J` panel inferior, `Ctrl+Shift+F` buscar,
  `Ctrl+S` guardar, `Ctrl+W` cerrar pestaña, `Ctrl+F` buscar en el archivo.

## Git en el Studio

La barra lateral tiene una vista **Git** (rail, menú Git, botón de la barra de
herramientas y `Git` en la barra de estado). Usa el `git` instalado, sin shell, sin pedir
credenciales y sin abrir ventanas.

- **Cabecera:** rama, Recibir (`pull --ff-only`; si se separaron ofrece fusionar), Enviar
  (con confirmación) o Publicar rama, y buscar novedades (`fetch`).
- **Cambios:** guía paso a paso según el estado (conflictos, listo para guardar, cambios,
  separadas, por enviar, sin publicar, sin commits, al día), caja de commit
  (`Ctrl+Intro`), y listas *En conflicto*, *Listos para guardar*, *Cambios* y *Apartados
  (stash)*. Un clic abre el diff en una pestaña de solo lectura; `+`/`−` preparan o quitan,
  `↶` descarta tras confirmar y guarda antes una copia (`changes.patch` y archivos nuevos)
  en `.git/chxchx-backups/` (se conservan las 10 últimas).
- **Historial:** commits de todas las ramas con grafo, etiquetas de rama y tag; un clic
  muestra el mensaje y los archivos, y otro abre el diff de ese archivo.
- **Ramas:** locales y remotas, cambiar, crear y eliminar (confirmación; nunca fuerza).
- **Explorador:** archivos coloreados y con letra (M, N, D, R, !) y un punto en las carpetas
  con cambios. El estado se refresca al guardar, tras cada acción y cada 8 s.
- **Gestión de archivos (explorador):** clic derecho o los atajos `F2` (renombrar), `Supr` (eliminar), `Ctrl+C` / `Ctrl+X` /
  `Ctrl+V`. Crear, renombrar, duplicar, copiar, mover y eliminar viven en `features/project_fs.rs`: todo dentro del proyecto,
  sin seguir enlaces, sin tocar lo que el explorador oculta (`.git`, `node_modules`, `target`, `.venv`, `__pycache__`, `.env*`),
  con nombres válidos en cualquier sistema y límites (20 000 elementos / 2 GiB al copiar). Eliminar no borra: mueve el elemento
  a `<home>/trash/<proyecto>/` y se deshace desde el explorador. Pegar un nombre que ya existe crea «nombre copia»; mover a un
  destino con ese nombre se rechaza. Los atajos solo actúan mientras el explorador tiene el teclado.
- **Seguridad:** rutas literales (`--literal-pathspecs`), nombres de rama validados con
  `git check-ref-format`, `core.fsmonitor` desactivado siempre, y en proyectos sin confianza
  local no se ejecutan los hooks del repositorio. Un solo comando que modifica corre a la
  vez, para no pelear por `index.lock`.
- **Sugerir y plan de commits (buenas prácticas):** el botón **Sugerir** mira lo que se va
  a guardar. Si son varios temas no propone un commit único: reparte los archivos
  relacionados en commits separados (el plan, abajo). Si es un solo tema, rellena la caja con
  un mensaje Conventional Commits (`tipo(alcance): descripción` y un cuerpo con los
  archivos y sus líneas). La caja de mensaje queda fija sobre la lista, que es la que
  desplaza.
  **Organizar en commits** «desfragmenta» todo lo pendiente en commits que cuentan una
  sola historia: clasifica cada archivo (docs, test, ci, build —dependencias primero—,
  chore, y código como feat, fix o refactor según archivos nuevos, tamaño del cambio,
  borrados y palabras como *fix/bug/corrige*), agrupa por módulo, subdivide los módulos
  grandes por carpeta y familia de nombres, y propone un mensaje por grupo. Cada tarjeta
  permite cambiar el tipo con un clic, editar la descripción y guardar solo ese grupo
  (`git commit -- <archivos>`: lo demás que estuviera preparado no se toca) o todos en
  orden. Avisa si el título supera 72 caracteres o no sigue el formato. Es un motor
  local: no envía nada a ningún servicio y no lee archivos que parecen secretos
  (`.env`, claves, tokens), que además se señalan con un aviso.
- **Sin portar aún:** consejo y sugerencia con un agente de IA, diff en dos columnas,
  recordar «no volver a confirmar el envío», rebase, cherry-pick y blame.

## Persistencia del Studio

El Studio guarda sus preferencias en `<CHXCHX_TECH_HOME>/studio-preferences.json`
(escritura atómica, diferida 1,5 s tras el último cambio y al cerrar): tamaño de
ventana, paneles visibles y sus tamaños, panel inferior, hasta 10 proyectos recientes
y, por proyecto, pestañas abiertas, pestaña activa, carpetas expandidas, sección de
Inicio y 20 archivos recientes. Los valores se acotan al cargar y las rutas deben
ser relativas al proyecto. Un archivo dañado se aparta como `.corrupt`; uno de una
versión más nueva no se sobrescribe. No se guardan secretos, confianza, terminales ni
agentes, y ninguna ruta guardada concede permisos.

## Rendimiento

La interfaz usa el renderer por software de Iced, optimizado: las dependencias se
compilan con `opt-level = 3` incluso en dev y el perfil release usa LTO. El trabajo
de disco (leer el proyecto, previsualizar, diagnosticar, aplicar) corre en hilos de
fondo y la barra de estado muestra «Procesando…». Se probó el renderer GPU (wgpu): en
la máquina de pruebas consumió más CPU (3,1×) y memoria (4×) al redibujar, así que
no se habilita. Para medir, compila con `--release` y no con el build debug.

La migración aún no cubre el cambio de proyectos, instaladores,
paridad automatizada completa ni QA en macOS/Linux.

Desde la raíz del repositorio:

```powershell
cargo run --no-default-features --features cli --bin chxchx-tech -- status . --json
cargo run --no-default-features --features cli --bin chxchx-tech -- init .
cargo run --no-default-features --features cli --bin chxchx-tech -- init . --apply
cargo run --no-default-features --features cli,tui --bin chxchx-tech -- tui .
cargo run --no-default-features --features cli,studio --bin chxchx-tech -- studio open .
cargo run --no-default-features --features studio --bin chxchx-studio -- .
```

## Verificación de CI

La suite de flujos compartidos se puede ejecutar sin compilar las interfaces
gráficas:

```powershell
cargo test --locked --no-default-features --features cli
python -m pip install --requirement scripts/requirements-rust-schema.txt
python scripts/validate_project_overview_schema.py
```

Las pruebas aplican operaciones solo dentro de directorios temporales. El
validador ejecuta `status --json` con un home temporal y comprueba la salida real
contra `schemas/project-overview-v1.schema.json`.

## Agentes y terminal en Studio

El panel **Agentes** (icono de destellos en la barra lateral, menú Ver → Agentes, chip en la barra de estado
y tarjeta en Inicio) lista los agentes del proyecto —los de `workspace.agents` de `.ai/chxchx-tech.toml` y
Claude Code, Codex y OpenCode que existan en el equipo— con su estado (disponible, instalado, en terminal) y
abre cada uno en una pestaña del panel inferior. Requiere confianza local en el proyecto.

La terminal del panel inferior es una terminal de verdad (emulación VT con colores, cursor, scrollback con la
rueda, teclado completo —flechas, Ctrl+letra, Alt, F1–F12—, pegado con Ctrl+Shift+V o Cmd+V y tamaño que sigue
a la ventana), así que las interfaces propias de los agentes funcionan dentro de Studio. Con el foco en la
terminal, Ctrl+J y Ctrl+Shift+<letra> siguen siendo atajos de Studio; el resto va al programa. Si Studio se
abrió desde un lanzador, los agentes se buscan también en `~/.opencode/bin`, `~/.local/bin`, nvm, Homebrew,
bun, npm global, etc.

### Chat visual

Cada agente con chat visual (Claude Code, Codex, OpenCode) tiene un botón **Chat** junto a **Terminal**, en el
panel Agentes y en la página de inicio. Abre una pestaña «Chat · agente» en el área central con la conversación
(Markdown), tarjetas de herramientas y razonamiento, **permisos en vivo** (permitir, permitir y recordar,
denegar), preguntas del agente, nivel de acceso (solo lectura, editar, editar y ejecutar, acceso total), red,
modelo y esfuerzo, **uso de tokens y coste**, contexto con su liberación automática al 90 % y límites del plan.
La conversación se guarda y se recupera al reabrir; «Nuevo chat» empieza de cero. Claude Code corre directamente desde
Rust. Codex y OpenCode los conduce el servicio `services/agent-adapter/` (Python, solo biblioteca estándar) mediante un
contrato versionado (`chxchx.agent-adapter` v1): Rust decide la política (trust, acceso, contexto) y el servicio solo
habla el protocolo en vivo de cada proveedor. Studio lo busca en `$CHXCHX_AGENT_ADAPTER`, junto al binario, en el
`.venv` de un checkout (`uv sync` en `services/agent-adapter`) y en el PATH. Sin confianza del proyecto no se inicia
ningún turno.

### Memoria explícita

Studio guarda en `.ai/memory/PROJECT_MEMORY.md` las solicitudes directas de recordar que aparecen en sus
transcripts visibles, omitiendo bloques inyectados y posibles secretos. Al abrir un chat y al iniciar **Nuevo chat**,
también recupera en segundo plano solicitudes explícitas desde hasta diez conversaciones recientes por proveedor,
Codex, Claude y OpenCode, del proyecto actual. Lee solo mensajes del usuario, con límites por transcript, y las
deduplica antes de agregarlas. OpenCode usa su CLI oficial con export sanitizado. Para revisar capturas automáticas
antiguas:

```bash
chxchx-tech workspace memory-prune . --dry-run # previsualiza sin escribir
chxchx-tech workspace memory-prune .            # aplica y guarda un backup fuera del proyecto
```

El comando solo evalúa secciones que ChxChx marcó como capturas automáticas; mantiene las notas manuales y
elimina entradas inválidas o duplicadas. Si la escritura falla, el original permanece respaldado en
`$CHXCHX_TECH_HOME/backups/` (o `~/.chxchx-tech-lead/backups/`).

## Windows

Studio de Rust funciona en Windows 10/11 x64 sin instalar nada más que el propio binario
(sin consola: usa el subsistema de ventanas y el icono va incrustado en el `.exe`).

- **Descargar sin compilar:** cada push a la rama genera el artefacto `chxchx-rust-windows-x86_64`
  (workflow *Rust product builds*, trabajo *windows-package*) con `chxchx-studio.exe`,
  `chxchx-tech.exe` y `studio-rust.ps1`. Antes de publicarlo, CI abre el Studio 8 segundos sobre un
  proyecto real y lee `status --json` con el CLI.
- **Compilar desde el checkout:** `./scripts/studio-rust.ps1 [-Project C:\ruta] [-Debug]` compila
  y abre el Studio de Rust por su ruta explícita. Importa porque el Studio Qt instala un lanzador
  con el mismo nombre (`chxchx-studio`) y, si está en el PATH, es el que se abriría.
- **Si no abre nada:** Studio no tiene consola, así que los errores de arranque y los pánicos se
  guardan en `%USERPROFILE%\.chxchx-tech-lead\logs\studio-startup.log` (o en
  `%CHXCHX_TECH_HOME%\logs\`). Ahí está el motivo.
- Primera compilación local: en una máquina de 8 núcleos tarda unos 40 minutos (las dependencias
  gráficas se optimizan incluso en `debug`); las siguientes son incrementales.

En la TUI, `i` muestra el plan de init completo, `m` el compacto, `s` muestra
sync, `y` aplica el plan abierto, `c` abre MCP, `d` ejecuta el diagnóstico y
`p` muestra proyectos registrados. En el preview de sync, `↑/↓` desplaza y `Esc`
cancela. `r` actualiza y `q` sale. Studio ofrece botones para init, sync,
rollback, trust, MCP, proyectos registrados y doctor. El shell de IDE recupera
la disposición Qt y el tema Medianoche: barra de actividad con iconos, accesos
rápidos arriba, explorador a la izquierda, archivos en pestañas centrales,
secciones a la derecha, panel inferior de Terminal/Actividad y barra de estado.
Las barras y paneles muestran el estado seleccionado y se pueden ocultar.

El editor abre texto UTF-8 hasta 2 MiB, resalta sintaxis, busca dentro del archivo,
ofrece deshacer/rehacer y guarda con respaldo fuera del repo. Rechaza conflictos
con cambios externos y pide confirmar antes de descartar ediciones. `Ctrl+S`
guarda, `Ctrl+F` abre la búsqueda, `Ctrl+Z` deshace y `Ctrl+Y` rehace. Se admiten
hasta 24 archivos; enlaces y archivos `.env*` quedan fuera del acceso/respaldo.
También se puede abrir un archivo relativo al iniciar:

```powershell
cargo run --bin chxchx-studio -- . README.md
```

Terminal abre un shell PTY/ConPTY solo tras conceder confianza local. La entrada
actual envía líneas y ofrece Ctrl+C; el panel puede ocultarse sin terminar la
sesión. El parser VT mantiene la pantalla y permite cambiar filas/columnas desde
el adapter; todavía faltan entrada interactiva completa, colores por celda,
scrollback navegable y accesibilidad. Al salir se cierra la terminal local.

La interfaz de Studio usa la marca ChxChx: `assets/brand/` contiene el isotipo y
el logo pre-escalados desde `assets/` de la raíz, más el `.ico` que `build.rs`
incrusta en el ejecutable de Windows (si falta el compilador de recursos, la
compilación continúa con un aviso). Los tokens de color y tipografía viven en
`studio_design.rs` y los estilos de widgets en `studio_styles.rs`; usa fuentes
del sistema, sin fuentes incrustadas. El explorador, el panel de secciones y el
panel inferior se redimensionan arrastrando su separador. `Ctrl+B` muestra u
oculta el explorador y `Ctrl+J` el panel inferior (Terminal / Actividad).

La paridad de Git visual, agentes/chat, edición avanzada, paneles acoplables y
preferencias sigue pendiente. Iced usa renderer software; esto sigue siendo un
port parcial de IDE, con capacidades operativas reales, no un reemplazo completo.
La compilación local se comprobó en Windows; macOS, Ubuntu y Fedora siguen
pendientes en CI/QA. La compatibilidad de JSON es una versión nueva e independiente
del bridge completo `chxchx.project-status` v1 del producto legado.


## Conexión: puertos, origen de las versiones y rutas

En **Studio → Ajustes → Conexión** (o con `chxchx-tech connection show|set|reset`) eliges lo que antes era fijo:
el puerto del nodo (47821), el puerto HTTPS de Tailscale (443), de dónde vienen las versiones y la clave pública
con la que deben estar firmadas, y dónde está cada herramienta (`claude`, `codex`, `opencode`, `python`,
`tailscale`) si no está en el PATH. Dejar un valor vacío es usar el de fábrica. Ejemplos:

```bash
chxchx-tech connection set --tool codex=/opt/codex/bin/codex --dry-run   # solo muestra
chxchx-tech connection set --https-port 8443
chxchx-tech connection reset                                             # el archivo anterior queda como .bak
```

La clave de firma es siempre la parte **pública**; la privada no se pega en ningún sitio de la aplicación.
