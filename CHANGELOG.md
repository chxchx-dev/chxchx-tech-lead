# Changelog

## Unreleased

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
