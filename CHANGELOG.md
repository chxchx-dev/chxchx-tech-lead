# Changelog

## Sin publicar

- **La app web probada en un teléfono real (Android, Chrome, modo oscuro y teclado)**: se corrigieron tres cosas que solo se veían ahí. Con el teclado abierto la cabecera con los agentes desaparecía y la barra de pestañas quitaba espacio: ahora el teclado reduce el área de la página, las pestañas se ocultan y los tres agentes siguen a la vista. Al abrir una conversación larga ahora baja al último mensaje (antes se quedaba arriba). Y en el chat se quitó la barra «Conversación» redundante (el punto de conexión pasó a la cabecera del chat) para ganar altura. Las etiquetas de estado de «Recientes» tienen el mismo ancho.

- **App web del teléfono con otro diseño**: el chat ocupa la pantalla; los tres agentes (Claude, Codex, OpenCode) quedan siempre visibles arriba, con un punto en el que está trabajando, de modo que cambiar de agente ya no obliga a subir toda la conversación. Solo se desplazan los mensajes; el cuadro de escribir queda fijo abajo, con el botón dentro. «Opciones» (acceso, modelo, esfuerzo) se abre desde el engranaje de la cabecera. Un solo sistema de espaciado: las tarjetas ya no tienen el texto pegado al borde (era un fallo de estilo en las tarjetas de proyecto), Inicio muestra tres cifras grandes y Proyectos una flecha que invita a entrar. 26 pasos de navegador en Chromium; falta verlo en tu teléfono, en modo oscuro y en Safari/iOS.

- **Vincular el celular es más cómodo**: el código se escribe en dos casillas, con el guion dibujado entre ellas (ya no hay que teclearlo). Al llenar la primera, el cursor salta a la segunda; Retroceso vuelve atrás; y puedes pegar el código entero (con guion, con espacios o en minúsculas) en cualquiera de las dos y se reparte solo. Las letras que un código nunca lleva (`I`, `O`, `0`, `1`) se descartan al escribir, para no gastar uno de los pocos intentos que permite el nodo. Lo que se envía es el mismo `XXXXX-XXXXX`. La pantalla de vinculación ahora también explica que el código sale de Studio (Ajustes → Nodo local).

- **El nodo y la app web vienen en los paquetes**: los instaladores y paquetes de Linux, Windows y macOS ya incluyen `chxchx-tech node ...` y la app para el teléfono que el nodo sirve (el CI la construye una vez y cada plataforma la incrusta; el paquete de Linux y el ZIP de Windows comprueban que el nodo está dentro). El nodo no arranca solo: lo inicias tú, desde Ajustes en Studio o con `node start`/`node service install`.

> La versión del producto es 0.7.0+ (nodo, app web, manager); el encabezado se renombra a «0.7.0» al publicar la estable.

- **Manager completo (H6)**: `chxchx-tech manager update [--apply]` aplica una versión nueva solo si el manifiesto está firmado (ed25519), el paquete coincide con su SHA-256 y es más nueva; espera a que no haya trabajos, conserva los archivos anteriores y los restaura si el programa no responde (solo Linux; hasta que haya clave de firma publicada no aplica nada). `manager watch` levanta el nodo y el servicio de agentes si se caen, con espera creciente y corte tras 3 reinicios por hora; `manager recover` repara copias viejas o rotas del servicio de agentes; `manager keygen` genera la clave de firma. El nodo ajusta cuántos turnos corre a la vez según tu memoria y CPU, y `manager status` muestra también el disco.

- **`chxchx-tech manager status|plan|apply` (primer paso de H6)**: dice qué tiene tu máquina (Claude, Codex, OpenCode, Python, servicio de agentes, nodo, Tailscale, memoria), qué falta y qué haría falta. `plan` solo muestra; `apply` hace únicamente lo que es de ChxChx y se puede repetir (desempaquetar el servicio de agentes, instalar el nodo como servicio de tu usuario) y deja claro lo que te toca a ti (Python, Tailscale); `--dry-run` no cambia nada. Con los mismos datos siempre da el mismo plan.

- **Codex y OpenCode sin pasos manuales**: el servicio que los conduce (`services/agent-adapter`, solo biblioteca estándar de Python) ahora viaja dentro del programa y se desempaqueta solo en la carpeta de ChxChx la primera vez que hace falta, con un lanzador que usa tu Python 3.11+. Ya no hace falta `uv sync`, ni un checkout, ni instalar nada aparte; si falta Python, el aviso lo dice claro. Un servicio instalado o definido con `CHXCHX_AGENT_ADAPTER` sigue teniendo prioridad.

- **Studio y el nodo (H5, en desarrollo)**: en Ajustes → *Nodo local* puedes iniciar o detener el nodo, ver sus dispositivos vinculados, crear un código para vincular otro y desvincular uno (con confirmación). Si activas «Usar el nodo para los turnos del chat», cerrar Studio ya no corta el turno: al volver a abrir lo recupera con su historial. `chxchx-tech node service install|status|uninstall` lo arranca al iniciar sesión como servicio de tu usuario (vista previa por defecto, `--apply` para instalar; nunca root). Probado de verdad en Linux; macOS y Windows sin verificar.

- **App web para el teléfono (nodo, en desarrollo)**: el nodo sirve una PWA (Dioxus, 610 KB comprimida) con tus proyectos, el chat con tus agentes en vivo, la aprobación de permisos desde el teléfono, la lista de trabajos y la gestión de dispositivos. Se instala desde el navegador, arranca sin red y se reconecta sola. `chxchx-tech node pair` imprime un QR cuando el acceso remoto está activado. Probada en Chromium con 25 pasos de extremo a extremo; **sin probar** en Safari/iOS ni Firefox. Ver `.ai/ROADMAP-HIBRIDO.md` (H4).
- **Acceso desde tus otros dispositivos por Tailscale (nodo, en desarrollo)**: `chxchx-tech node remote enable` muestra una vista previa y, con `--apply`, hace que el nodo responda en `https://tu-maquina.tailnet.ts.net/` con `tailscale serve`, solo dentro de tu red privada. Nunca usa Funnel, no cambia puertos que ya sirvan otra cosa y `disable` lo deshace. El token local no vale por la tailnet: cada dispositivo se vincula con su código. La revisión de seguridad está en `.ai/NODE-SECURITY.md`.
- **Dispositivos vinculados (nodo, en desarrollo)**: `chxchx-tech node pair` crea un código de un solo uso; el celular lo canjea por su propio token (roles `reader`, `operator` y `admin`), que se puede revocar al instante con `node revoke`. Un dispositivo nunca pasa de `workspace-write` ni usa la red salvo que sea `admin`, y `node audit` cuenta quién hizo qué sin guardar secretos.
- **Nodo local (en desarrollo, `--features node`)**: `chxchx-tech node start|status|stop` sirve los trabajos en `127.0.0.1` con token y eventos en vivo por WebSocket. Las tarjetas de permiso y las preguntas del agente esperan tu respuesta por la API, hasta 10 minutos. Rechaza peticiones que no vayan dirigidas a loopback y, al pararse, cancela lo que esté en curso. En reposo usa unos 10 MiB. Todavía no entra en los instaladores.
- **Hablar con un agente sin abrir Studio** (`chxchx-tech jobs run --project ALIAS --provider claude|codex|opencode "mensaje"`): el turno corre como un trabajo guardado. Si el comando se corta, queda `interrupted` con su historial en vez de perderse. `jobs list` y `jobs show ID --events` lo consultan después, y la conversación aparece en Studio como si la hubieras escrito allí. Es el primer paso para seguir trabajando desde el celular (ver `.ai/ROADMAP-HIBRIDO.md`).
- **Marcas de los agentes**: Claude, Codex (OpenAI) y OpenCode se reconocen por su marca en el panel de agentes y en los chats, en vez de una letra. Los dibujos vienen de simple-icons (CC0) y del repositorio de OpenCode (MIT); ver `assets/brands/README.md`.
- **Catálogo para desarrollar y operar modelos de lenguaje** (40 skills, 22 packs, 31 mods): entrenar desde cero, datos, tokenizador, ajuste fino, evaluación, inferencia y servicio, PyTorch, rendimiento en GPU, reproducibilidad, validación de seguridad y RAG; además Rust, Go, Kubernetes, Terraform, observabilidad, ingeniería de datos, CI/CD y accesibilidad. Nuevos packs (`llm-from-scratch`, `llm-fine-tuning`, `llm-inference`, `llm-applications`, `rust-service`, `go-service`, `kubernetes-platform`, `terraform-infrastructure`, `data-platform`) y mods de guardia (`llm-training-guard`, `llm-eval-gate`, `data-validation-gate`, `model-release-checklist`, `inference-benchmark`, `kubernetes-safety`, `terraform-plan-first`, `accessibility-check`).
- **Detección de proyectos**: nuevos stacks `ml` (entrenan o sirven modelos: PyTorch, Transformers, vLLM, wandb…), `kubernetes` (Helm, kustomize o manifiestos) y `terraform`, para que el catálogo recomiende lo adecuado y no a todo proyecto Python.
- **El explorador organiza el proyecto**: clic derecho en un archivo, una carpeta o el espacio vacío abre un menú con *Nuevo archivo*, *Nueva carpeta*, *Renombrar*, *Duplicar*, *Copiar*, *Cortar*, *Pegar*, *Copiar nombre / ruta / ruta relativa*, *Mostrar en el explorador de archivos* y *Eliminar*. Renombrar y crear se escriben en la propia fila (Enter confirma, Escape cancela) y validan el nombre mientras escribes. Ctrl+clic y Mayús+clic seleccionan varios elementos. `F2`, `Supr`, `Ctrl+C`, `Ctrl+X` y `Ctrl+V` actúan solo cuando el explorador tiene el teclado y se pueden cambiar en Ajustes. Eliminar mueve a una papelera de ChxChx con confirmación y se puede deshacer (las últimas 10 eliminaciones). Las pestañas abiertas y las carpetas expandidas siguen a los renombrados y movimientos. Nada sale del proyecto, no se siguen enlaces ni se tocan `.git`, `node_modules` o `.env`, y pegar nunca sobrescribe. También se puede **arrastrar** para mover (con `Ctrl` copia), **soltar archivos desde el gestor de archivos del sistema** y pegar los que el sistema tenga copiados.
- **Menús más completos**: *Archivo → Abrir proyecto…* (selector de carpeta, en esta ventana o en una nueva, `Ctrl+O`), *Abrir reciente*, *Nuevo archivo / carpeta* (`Ctrl+N`), *Guardar todo*, *Cerrar todas las pestañas*, *Copiar la ruta del archivo* y *Mostrar en el explorador de archivos*; *Editar → Cortar / Copiar / Pegar / Seleccionar todo*; *Ver → Acercar / Alejar / Tamaño normal* (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`) y *Pantalla completa* (`F11`). Todos los atajos nuevos se pueden cambiar en Ajustes.

## 0.6.3 — Ajustes, temas y un Studio más fluido

- **Ajustes** (*Archivo → Preferencias*, o `Ctrl+,`): seis temas (ChxChx Oscuro, Medianoche, Grafito, Bosque, Claro y Alto contraste), color de resalte propio, tamaño de la interfaz del 90 % al 150 % y atajos de teclado que se pueden cambiar o restaurar. Todo se guarda en `studio-preferences.json` y se aplica al instante.
- **Chats visuales**: `↑` / `↓` recorren los mensajes que enviaste; cada mensaje se puede copiar o pasar a texto seleccionable (`Ctrl+C`); `Ctrl+Mayús+Y` copia la última respuesta; los mensajes enormes se pliegan; con mucho texto ya no se traba (el Markdown solo analiza lo nuevo y no se clonan las respuestas en cada fotograma).
- **Modelos**: Claude muestra también las versiones concretas (`claude-fable-5-2`, `claude-opus-5-5`…) y los modelos que ya usas; OpenCode espera más a `opencode models` y añade los modelos de tu `opencode.json`.
- **Agentes**: los botones *Terminal* y *Chat* ya no se cortan en las tarjetas.
- **Scroll de páginas más ligero** (Inicio, Agentes, Ajustes…): unos 8 ms de CPU por paso de rueda en lugar de unos 16 (−50 %). Las tarjetas altas con esquinas redondeadas solo suavizan las esquinas, la máscara de recorte se reescribe solo donde cambia y los degradados de botones y de la tarjeta de bienvenida son ahora un color liso.
- **Barras renovadas**: la barra de herramientas se agrupa en secciones (con subrayado en la activa), archivo y búsqueda, con Ajustes y Actualizar a la derecha; los tooltips y los atajos del menú siguen tus atajos personalizados; en la barra de estado, la rama abre Git y los permisos abren Proyecto y permisos.
- **Conectores MCP**: catálogo de servidores para Excel, Power BI, Word, Google Sheets, PostgreSQL y SQLite, más servidores propios. `chxchx-tech connectors list|add|remove|custom`, la página *Conectar* del TUI y la sección *Herramientas / MCP* de Studio. Se escriben en los archivos del proyecto de cada agente (`.mcp.json`, `.codex/config.toml`, `opencode.json`) con vista previa exacta, respaldo de lo que existía y sin guardar claves.

## 0.6.2 — Scroll más ligero

- **Desplazarse por el editor y por los chats largos cuesta menos**: unos 8 ms de CPU por paso de rueda en lugar de unos 13
  (−39 %). Antes, la tarjeta redondeada que envuelve el editor se pintaba entera, con suavizado, en cada fotograma; ahora los
  rectángulos lisos y los redondeados opacos se rellenan como rectángulos y solo las esquinas llevan suavizado.
- El editor ya no copia el documento completo para compararlo al desplazarse o mover el cursor; solo lo hace al editar.
- Una carrera que podía perder el último mensaje de un turno de chat o de una instalación de modelo local queda corregida.

## 0.6.1 — Inicio más claro

- La guía de Inicio se concentra en tres pasos: preparar el proyecto, decidir la confianza local y empezar a trabajar.
- Al terminar una vista previa, Studio desplaza la página hasta el panel de revisión. La confianza explica que guardarla no ejecuta procesos y que se puede revocar.
- Inicio ocupa menos espacio vertical. La lentitud general del scroll en otras páginas sigue pendiente de medición en la ventana.

## 0.6.0 — el producto pasa a Rust

Reescritura completa del producto en Rust: un solo comando, tres interfaces y mucho menos consumo.
La instalación 0.5.x se actualiza desde **Ayuda → Buscar actualizaciones…** (o con `install.sh` / `install.ps1`).

### Un solo comando
- `chxchx-tech` (y su nombre corto `chxchx`) abre todo: Studio si hay escritorio, el TUI en una terminal, la ayuda en
  una tubería. `chxchx-tech ~/mi-proyecto` abre esa carpeta.
- Atajos `st`, `dr`, `sk`, `ws`, `pr`, `t`, `gui`; los comandos del CLI anterior (`agent`, `bridge`, `local-models`…)
  también se escriben con `chxchx-tech`.

### Studio (escritorio, nativo)
- **Chat visual** con Claude Code, Codex y OpenCode: permisos en vivo, niveles de acceso, imágenes (`Ctrl+V`),
  selector de modelos con buscador, esfuerzo, tokens/coste/contexto y comandos `/` (acciones del chat y skills del proyecto).
- **Modelos locales (Ollama):** diagnostica tu equipo, recomienda lo que cabe, instala y conecta a OpenCode con progreso,
  y se usan como cualquier agente (chat o terminal).
- **Paneles y pestañas** como en VS Code: chats, terminales y editor se arrastran, dividen y unen.
- **Terminal real** (pantalla VT completa, teclado, varias sesiones).
- **Editor** con tipo de archivo, vista previa de Markdown, y formateo con la herramienta del proyecto (Prettier,
  rustfmt, ruff, gofmt), obligatorio al guardar solo si el proyecto lo pide.
- **Git** con historial dibujado, plan de commits y ramas; **catálogo** de skills, packs y mods.
- Ruta de aprendizaje en Inicio, diseño minimalista y banners ASCII por agente.
- **Ayuda → Buscar actualizaciones…** actualiza sin duplicar instalaciones.

### TUI rediseñado
- Ocho páginas (Inicio, Preparar, Catálogo, Agentes, Git, Conectar, Salud, Proyectos) con pestañas, clic y ayuda.
- Toda acción abre primero una vista previa; `Intro` en un agente lo abre en la misma terminal.

### CLI con color
- En una terminal: tarjetas, tablas y símbolos; redirigido o con `NO_COLOR`, el texto plano de siempre. `--json` nunca se decora.

### Rendimiento y memoria
- ~40 MB de RAM en reposo, sin temporizadores fijos: duerme hasta que hay algo que hacer. Ver [MEMORIA.md](MEMORIA.md).

### Instalación y actualización
- Una sola instalación por canal: al actualizar se retiran las anteriores (ya no se acumulan `ChxChx Studio 0.5.x`).
- Los instaladores y el actualizador dejan `chxchx-tech` y `chxchx` apuntando al producto nuevo.

### Para quien contribuye
- La versión anterior (Python + C++/Qt) sale del repositorio; queda solo el CLI de Python que aún sirve el chat, los modelos
  locales y el actualizador, hasta que se porten a Rust.
- Pruebas de extremo a extremo del CLI y de Studio, y del TUI con una pantalla en memoria.

## Sin publicar antes de 0.6.0 (incluido en 0.6.0)

- **El chat visual conserva la conversación al cerrar.** Después de cada turno (y al empezar, para no perder tu mensaje si cierras a mitad), `agent chat` guarda en `.ai/chat-state/<agente>.json` el id de sesión y los últimos 80 mensajes de texto, sin secretos. Al abrir el chat, Studio los restaura con `agent chat-state --restore` y sigue con `--session`; «Nuevo chat» olvida el estado (`--reset`). Si el agente ya no conserva esa sesión, se comprueba una vez y la conversación guardada viaja en el primer mensaje de una sesión nueva, así el hilo no se pierde. Un traspaso de la liberación de contexto sobrevive a cerrar antes del siguiente mensaje (se usa al reabrir y se borra solo cuando la sesión nueva ya existe).
- El traspaso de contexto se escribe de forma atómica (un cierre a mitad no lo deja truncado), sin líneas que parezcan secretos, y el agente debe incorporar el traspaso anterior para que encadenar liberaciones no pierda datos.
- Corrección: `mod list --json`, que lee la pantalla de Mods de Studio, partía el JSON en varias líneas dentro de las cadenas cuando era largo (la consola ajusta a 80 columnas). Igual en `ci` y `deploy`.

- **Aprobaciones y preguntas en vivo con OpenCode 2.x.** El chat visual vuelve a pausarse y preguntar al instante también con la 2.x, con un controlador nuevo sobre su API local (`/api/session`, `/prompt`, eventos `/api/event`, respuestas de permisos y formularios). Mismas tarjetas que con la 1.x: «permitir esta vez», «permitir `git *` en este chat» (se vuelve la regla `bash:git *`), denegar (el paso se interrumpe, sin mostrarlo como error) y preguntas con opciones, selección múltiple y respuesta libre. La sesión se reanuda por id, el modelo y el esfuerzo viajan en la referencia del modelo y el nivel de acceso entra como configuración del servidor del turno. Se elige por la versión instalada (1.x sigue con su controlador). Verificado con OpenCode 2.2.6 real (aprobar, denegar, pregunta, regla pre-aprobada, reanudar, cancelar sin procesos sueltos) y con un servidor falso de la 2.x (6 mutaciones, 6 detectadas).

## 0.5.4 - 2026-11-10

- **Alta de agentes sin preparar el proyecto**: `agent add` ya no falla si falta `.ai/chxchx-tech.toml` (caso típico de OpenCode en un proyecto nuevo). Ejecuta solo `init --minimal` (únicamente el toml y `.ai/memory`, con backup, sin tocar `AGENTS.md` ni `CLAUDE.md`) y añade el agente; `--dry-run` solo previsualiza. Es idempotente.
- **Inicio con «Siguiente paso recomendado»** en Studio: lista preparar → confiar → agente → skills → abrir, marca lo ya hecho según el estado real y un botón ejecuta el paso actual con las vistas previas y confirmaciones existentes.
- Tras confirmar `agent add`, Studio abre la vista previa de confianza (la configuración cambia el hash); el trust sigue siendo una confirmación explícita.
- **Registro de fallos de arranque de agentes**: si un agente no inicia, Studio muestra el motivo exacto (`No se pudo iniciar <agente> (<programa>): <error>`) y lo guarda en la caché de errores (`bridge report-error`, operación `studio:agent-start:<id>`), visible en **Más → Errores**. Las credenciales se redactan.

## 0.5.2 - 2026-11-07

- **Liberación automática de contexto** en el chat visual (Claude, Codex y OpenCode). Al terminar un turno con el contexto en el 90 % o más, el agente escribe un traspaso de la conversación (objetivo, decisiones, lo hecho, estado, pendiente, datos exactos) en esa misma sesión y en solo lectura; el siguiente mensaje arranca una sesión nueva que recibe el traspaso junto con los archivos de contexto del proyecto y sigue sin que se note (un solo aviso discreto). El traspaso vive en `.ai/chat-handoffs/` y se borra al usarse. `agent chat --no-auto-release` lo desactiva. Para modelos de OpenCode que no informan su ventana se lee de su configuración o de su caché de modelos, y Studio ya recibe la ventana para mostrar el porcentaje. Ver ADR 0015.
- **OpenCode 2.x**: la 2.x cambió flags, permisos y la API del servidor local, y rompía el chat visual (en instalaciones nuevas: «No pude hablar con el servidor de OpenCode: HTTP 405»). Ahora se detecta la versión: con la 2.x los turnos usan `run --standalone`, el esfuerzo viaja en el modelo (`proveedor/modelo#esfuerzo`), los niveles de acceso se traducen a su lista ordenada de `permissions` (la 2.x ya no respeta `OPENCODE_PERMISSION`, así que antes «solo lectura» no impedía ejecutar comandos) y las aprobaciones llegan como tarjetas entre mensajes; la memoria y el diario leen `session export`. Las aprobaciones en vivo de OpenCode siguen disponibles solo en la 1.x.
- **Agentes fuera del PATH de la terminal**: Studio abierto desde el menú o el Dock tiene un PATH mínimo y no veía `opencode`, `codex`, `claude` ni `node`. Se buscan también en sus carpetas de instalación (`~/.opencode/bin`, `~/.local/bin`, nvm, Homebrew, bun, npm global…) y los agentes se lanzan con ese PATH. Si un agente de terminal no arranca, la terminal dice por qué.

## 0.5.1 - 2026-11-06

- Git en Studio: **Sugerir** entiende los cambios y los agrupa por tema en varios commits (código con sus pruebas, documentación, CI…) en lugar de un solo commit con todo. Muestra un plan; **Preparar** deja en el área de preparación solo los archivos de ese grupo y rellena su mensaje, y al guardar el siguiente queda a la espera. Los archivos que el asistente omita van a un último grupo; los inexistentes se descartan; los sensibles nunca se envían. Con un solo tema, el comportamiento es el de antes.
- Los Mods instalados (`.claude/skills/` y `.agents/skills/`) ya no se versionan en este repositorio: se regeneran en cada PC con `mod apply` o desde Studio.
- Mods para todos los agentes: además de Claude Code, los Mods llegan a **Codex** (`.agents/skills/`, que Codex lee; sin esa carpeta no ve ninguno) y a **OpenCode** (lee `.claude/skills/`, sin copia extra). Se instalan en la carpeta de cada agente configurado, `mod remove` los retira de todas con backup, y la preparación de proyectos los ofrece si hay cualquiera de los tres agentes.
- Mods: el catálogo pasa de 9 a 22. Nuevos Mods por stack (Python, React/Next.js, NestJS, React Native, bases de datos, Redis, Docker, .NET, Rust, Go, aplicaciones con LLM), generales (revisión de seguridad, estrategia de pruebas) y GitHub Actions, que se recomienda por la infraestructura detectada (`infrastructure` en `mod.toml`). `mod apply` y Studio instalan los recomendados con un paso.
- Chat visual con OpenCode: esperar tu respuesta a un permiso ya no cuenta como «dejó de responder».
- Chat visual con OpenCode: aprobaciones y preguntas en vivo mediante `opencode serve` (HTTP y eventos del servidor): `permission.asked` y `question.asked` pausan el turno y Studio responde. El servidor es local, efímero, en un puerto libre y con contraseña por turno; «permitir en este chat» usa el patrón que propone OpenCode (`bash:git *`). Un proyecto sin archivos de contexto ya no manda al agente a buscarlos. Los tres agentes del chat visual son ahora en vivo.
- Chat visual con Codex: aprobaciones y preguntas en vivo mediante `codex app-server` (JSON-RPC): comandos, cambios de archivos, permisos extra (red, carpetas) y `request_user_input`, con «permitir esta vez», «permitir mientras dure el turno» o denegar. El nivel de acceso se traduce a sandbox y política de aprobación (solo *Editar archivos* pregunta). Uso, sesión y contexto del turno se calculan desde las notificaciones del servidor. OpenCode sigue con tarjetas entre mensajes.
- Chat visual: un chat nuevo ya no pide permiso para leer el contexto. Studio incluye en el primer mensaje los archivos de contexto (`AGENTS.md`, `.ai/PROJECT.md`, `CURRENT_STATE.md`, `HANDOFF.md`, memoria del proyecto; con tope de tamaño, sin seguir enlaces simbólicos, y Codex omite `AGENTS.md` porque ya lo carga) y la captura de memoria los ignora.
- Chat visual con Claude: aprobaciones y preguntas **en vivo**. El agente se pausa y Studio muestra la tarjeta de permiso o de pregunta (opciones como botones, varias preguntas, selección múltiple y respuesta libre); al responder, el turno continúa. Usa el protocolo oficial `--input-format stream-json --permission-prompt-tool stdio`; `agent chat --live` (Studio lo envía siempre) y respuestas JSON por stdin. Codex y OpenCode siguen con tarjetas entre mensajes. Ver ADR 0014.
- Chat visual: seguridad y acceso. Cuatro niveles (solo lectura, editar archivos, editar y ejecutar, acceso total con confirmación), interruptor de red y **tarjetas de permiso** cuando el agente queda bloqueado (Claude y OpenCode con certeza, Codex como posible bloqueo del sandbox) con «una vez», «en este chat» o «ahora no», y reintento automático. `agent chat` gana `--network` y `--allow`; el modo por defecto de OpenCode ahora pide permiso para comandos y web. Ver ADR 0014.
- Instaladores por sistema con canal estable (`vX.Y.Z`) y `dev`: `install.sh` (Linux/macOS, curl) e `install.ps1` (Windows) con `--version`, `--update`, `--cli-only`, `--uninstall` y `--dry-run`; verifican el SHA-256 del manifiesto, instalan sin administrador en carpetas por versión (se conserva la anterior) y no tocan nada ajeno. Linux añade entrada en el menú de aplicaciones; Windows, menú Inicio y *Aplicaciones instaladas*. `chxchx-studio --version`. CI: manifiesto con `release_assets.py`, release estable por tag, smoke de instaladores contra un servidor local y firma (Windows/macOS) preparada pero desactivada. El actualizador de Studio sigue el canal instalado y ya lee `schema_version` del manifiesto (antes lo rechazaba). `install.sh`/`install.ps1` del checkout pasan a `install-from-checkout.*`. Ver ADR 0013.
- El asistente de Git usa por defecto un modelo ligero con esfuerzo bajo, elegido de la lista que publica cada agente; el menú del agente permite elegir otro modelo o el predeterminado del agente.
- Git en Studio: asistente con los agentes del proyecto. Una tarjeta guía el siguiente paso por reglas (sin agente) y, con uno conectado, sugiere el mensaje del commit y da consejo guiado ante ramas separadas, envíos rechazados, credenciales y conflictos. Solo propone; no envía archivos sensibles, enmascara secretos y pide confirmación la primera vez. Nuevo `agent chat --lean` / `--prompt-file`. Rediseño del panel con los iconos de Studio.
- Git en Studio, fase 2: botón de sincronización (recibir, enviar, publicar, traer novedades) con cancelación, avance en línea recta por defecto y fusión opcional con opción de cancelarla; nunca se fuerza un envío ni se piden credenciales. Apartados (stash) dentro de Cambios y estado de Git en el explorador de archivos (color, letra y punto en carpetas).
- Studio añade la pestaña **Git** (`Ctrl+Shift+G`): cambios con preparar/quitar/descartar y commit en lenguaje claro, diff lado a lado o en línea como pestaña del editor, historial con grafo de ramas y detalle del commit, y ramas (cambiar, crear, borrar). Descartar guarda antes una copia en `.git/chxchx-backups/`. Motor nativo y asíncrono; ver ADR 0011.
- El selector de agentes de Inicio ofrece **＋ Añadir OpenCode** cuando el proyecto no lo tiene (misma vista previa y confirmación que `agent add`). README con OpenCode y modelos locales, y capturas de ejemplo en `assets/screenshots/`.
- Pruebas en capas (`tests/README.md`): servidor HTTP falso de Ollama y ejecutables falsos de `ollama`/`opencode` para flujos E2E, y pruebas `live` opt-in (`pytest -m live`) contra las herramientas reales. Los agentes se lanzan por ruta completa para que los shims `.cmd` de Windows funcionen.
- Modelos locales: `local-models remove` (borra el modelo y sus variantes de Ollama y los quita de `opencode.json`), modelos cargados en memoria (`/api/ps`), botón Quitar en la pestaña de Studio y acceso desde Inicio.
- Studio añade la pestaña `Herramientas → Modelos locales (Ollama + OpenCode)…`: hardware, estado de Ollama, encaje de cada modelo, vista previa, descarga con confirmación y progreso, sobre `bridge local-models` y `local-models setup --json`.
- Añade `local-models status|setup` y `bridge local-models`: detecta RAM, GPU y disco, estima qué modelos de Ollama caben, los descarga con confirmación, crea una variante de 32K de contexto y los conecta al `opencode.json` del proyecto con `--dry-run` y backup propio. ChxChx no instala Ollama.
- Añade OpenCode como tercer agente del chat visual (`opencode run --format json`), con modos de solo lectura y edición, modelo, imágenes, uso y costo, y su sesión en el diario `.ai/history/SESSIONS.md`. Codex, Claude y OpenCode comparten `AGENTS.md`, `.ai/` y la memoria del proyecto, y cada uno funciona por separado.
- Identifica el build de pruebas de Terminal Workspace como `0.5.0.dev0` y documenta cómo actualizar instalaciones existentes desde `dev`.
- Escribe el estado persistente mediante archivo temporal y reemplazo atómico para tolerar interrupciones.
- Rota los logs de procesos al superar 5 MiB y conserva hasta tres copias anteriores.
- Añade pruebas de recuperación del estado de procesos tras reiniciar el manager.
- Actualiza roadmap y guía: la validación multiplataforma y la evidencia de uso real son los pendientes para v1.0.

- Renombra el proyecto, paquete y CLI a `chxchx-tech`, incluyendo su configuración, estado global, instaladores y documentación.
- Introduce el dominio inicial de Terminal Workspace y la configuración `chxchx-tech.toml` v2.
- Migra configuraciones v1 de forma idempotente y valida comandos, rutas relativas e IDs antes de cualquier ejecución.
- Añade confianza local y estado de workspace fuera del repositorio, con ejecución protegida de procesos.
- Añade el Process Manager y los comandos CLI iniciales `workspace status/trust` y `process list/start/stop`.
- Añade adapters aislados para Codex/Claude/OpenCode, Docker Compose y Git de solo lectura.
- Conecta el servicio de workspace con `open/start/stop/attach`, `editor open` y `agent start`.
- Añade `chxchx-tech resources` con RAM, swap, CPU, consumo de procesos y umbrales sin auto-kill.
- Añade dashboard TUI inicial con Textual, refresco de estado y acciones básicas del workspace.
- Fija el servidor MCP de Basic Memory al proyecto local para compartir conocimiento entre clientes.
- Añade `chxchx-tech agent start --all` y un header compacto configurable para sesiones Zellij.
- Añade banners ASCII por pane, reactivación de comandos suspendidos al adjuntar y orientación horizontal/vertical de agentes.
- Añade la receta `chxchx-tech run` para encadenar start, agentes y attach, más atajos raíz `attach` y `stop`.
- Actualiza los instaladores para preparar `uv`, ChxChx, Basic Memory y Serena de forma idempotente.
- Amplía `doctor`, README y la guía de uso con el flujo completo de instalación, trust y pruebas del workspace.
- Amplía la TUI con pestañas de resumen, proyectos, agentes, procesos, recursos y handoff, además de paleta `Ctrl+P` y acciones operativas.
- Completa las acciones del TUI para iniciar todo, controlar agentes/procesos, actualizar recursos y editar el handoff; añade el sello ASCII de CHXCHX-DEV.

## 0.2.0 - 2026-09-23

- Endurecimiento de `--dry-run`, backups, rollback e idempotencia.
- Perfiles TOML configurables y detección de PostgreSQL/AI.
- Comandos `projects list` y `projects sync`.
- Verificación posterior de integraciones MCP.
- Suite ampliada y smoke test de laboratorio.

## 0.1.0

- CLI inicial.
- Detectores básicos de stack.
- Estructura `.ai/` y documentación.
- Bloques administrados para AGENTS/CLAUDE.
- Instalación inicial de Basic Memory y Serena.
- Integración MCP inicial Claude/Codex.
- Diagnóstico, status, sync y rollback.
