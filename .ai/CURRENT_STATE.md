# Current State

## Estado actual

- Transporte PTY/ConPTY añadido a `native/src/integrations/pty_session.*`: shell o proceso por argv/cwd, entrada, salida asíncrona, resize y cierre. POSIX usa `forkpty` (Linux/macOS); Windows usa ConPTY, mínimo Windows 10.
- `native-pty-smoke` comprueba un shell interactivo: Studio envía una línea al proceso y cambia la geometría; en Linux también valida que `stty size` devuelve el nuevo tamaño. CMake compila y CTest pasa 2/2 en Fedora. El backend ConPTY debe quedar validado por el runner de Windows CI.
- Commit `a5c0753` integra pestañas de agentes con PTY/ConPTY y libvterm. Los cambios locales añaden `agent preflight` para trust/RAM agregado y pasan el arranque embebido por `agent_pane_command`, incluyendo instrumentación y el prompt contextual de sesión nueva. Python: 195 passed, 1 skipped. La compilación nativa sigue sin comprobarse porque CMake no puede resolver GitHub para descargar Scintilla/Lexilla/libvterm, aunque Qt6 Core5Compat ya está instalado.
- Cierre del bloque actual: Recursos de Studio consume `bridge resources PATH`, con muestra global del sistema y resumen de procesos gestionados para cada proyecto registrado. El endpoint es read-only y una prueba verifica que conserva el archivo de configuración.
- El editor usa Scintilla/Lexilla fijados a commits upstream; tiene UTF-8, guardado atómico, lexer por extensión y búsqueda en archivo. La paleta abre terminales externas para `workspace terminal`, `workspace attach` y `agent attach`, todas con preview/confirmación; el attach de agentes repite el RAM Governor.
- Validación actual de Python: `python -m compileall -q src tests scripts`; `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest -q` (195 passed, 1 skipped); `git diff --check` limpio. Qt6 Core5Compat está instalado (`qt6-qt5compat-devel-6.11.2`). CMake detecta Qt, pero no completa FetchContent porque este entorno no resuelve `github.com` al descargar Scintilla.
- Requisito local: el build estándar usa Qt6 Core5Compat de desarrollo; ya está instalado en este Fedora. CI instala el módulo desde sus runners.
- Pendiente inmediato: implementar scrollback acotado/navegable y pruebas de pantalla ANSI/VT; compilar Studio cuando haya acceso a las dependencias fijadas de GitHub. ADR-0001 local ya refleja la decisión aprobada de incorporar editor y terminal nativos en Studio, manteniendo CLI/TUI independientes y el launcher externo como fallback. Después: búsqueda global, recientes, splits, Git diff, símbolos/LSP, instaladores, QA visual y métricas en Windows/macOS/Linux.
- Studio añade vistas funcionales de Configuración, Guía y Marca. Setup ofrece preparación completa, init completo/mínimo, instalación de herramientas e integración MCP; todas las mutaciones pasan por dry-run y confirmación. Doctor es solo lectura. `Ctrl+P` filtra vistas, archivos y acciones disponibles; `Ctrl+F` busca en el archivo abierto.
- CI ahora compila Qt en Ubuntu, Windows, macOS y Fedora. Build local Qt 6.11.2, compileall y suite: 191 passed, 1 skipped. `setup --dry-run` y `doctor` se ejecutaron correctamente sin cambios.
- La rama activa es `dev`.
- Primer corte de ChxChx Studio creado en `native/`: aplicación C++20/Qt 6 Widgets con árbol de proyecto, apertura de archivos, pestañas, guardar atómico y vistas con consultas asíncronas para Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs, Recursos, Handoff, Memoria, Chats y Errores.
- Añadido `chxchx-tech bridge status PATH`, contrato JSON `chxchx.project-status` versión 1 con metadatos de proyecto, trust/workspace, agentes, procesos, RAM/swap/CPU y política RAM Governor. El snapshot evita persistir cambios de estado de procesos; Studio lo consume y presenta vistas resumidas por área.
- Lecturas de Inicio, Proyecto, Agentes, Procesos y Recursos ya usan el bridge JSON en vez de parsear tablas Rich del CLI. `bridge status .` devuelve JSON válido en ejecución local.
- Agentes y Procesos tienen selector y acciones nativas: iniciar agente, iniciar todos, nuevo chat con contexto, iniciar/detener proceso. Cada acción primero corre en `--dry-run` y pide confirmación; el inicio real de agentes vuelve a pasar por el RAM Governor y solo ofrece `--force` tras mostrar el aviso actualizado y pedir confirmación.
- El bridge ahora incluye procesos configurados detenidos para que se puedan seleccionar e iniciar. La navegación que cambia mientras hay una consulta en curso queda en cola y se actualiza al terminar.
- Workspace ahora ofrece trust, iniciar/reanudar, suspender y detener procesos con preview/confirmación. Proyectos muestra los registros disponibles y puede cambiar el workspace con `projects switch --no-attach`, actualizar raíz del árbol de archivos y cwd de Studio, y repetir la confirmación del RAM Governor con el estado vigente.
- Proyectos también permite dar trust al proyecto elegido antes de cambiar; deshabilita Switch para rutas no confiables y para el proyecto actual.
- `bridge status` incluye proyectos registrados, estado, trust y existencia de cada ruta. El payload JSON v1 se amplió de forma aditiva y reporta configuración inválida sin ocultar la lista multiproyecto.
- Skills y Tech Packs ya usan ese bridge: Studio muestra las 17 skills, selección por proyecto, seis recomendaciones para el stack actual y los 11 packs con composición/motivos de detección. Permite habilitar/deshabilitar una skill, aplicar un pack o los detectados y sincronizar el contexto; todas las mutaciones hacen preview `--dry-run` y piden confirmación antes del comando real.
- Una prueba nueva cubre catálogo, recomendaciones, composición de packs y reflejo de selección activa en el payload. Validación: `python -m compileall -q src tests`, suite (`189 passed, 1 skipped`), `git diff --check HEAD` y dry-runs de `skill enable`, `skill sync` y `pack apply-detected` pasan sin cambios reales.
- Qt 6.11.2 quedó instalado en el host Fedora. `cmake -S native -B /tmp/chxchx-studio-build -G Ninja` configura y `cmake --build /tmp/chxchx-studio-build --parallel 2` compila y enlaza. El primer build reveló el uso inválido de `QFileSystemModel::isFile`; se cambió a `!isDir` y el build final pasó.
- Handoff y Memoria ya tienen vistas nativas propias. Handoff carga el documento, precarga los campos editables cuando encuentra el formato administrado y guarda resumen/pendiente/validación mediante `workspace handoff` tras preview y confirmación. Memoria filtra notas por título/contenido y muestra su contenido como solo lectura.
- Se agregaron lecturas acotadas `bridge handoff` y `bridge memory`; las rutas resueltas deben permanecer dentro del proyecto. Se validaron ambos contratos y `workspace handoff --dry-run`. Suite de ese bloque: `190 passed, 1 skipped`.
- Chats y Errores ya tienen vistas nativas de solo lectura. Chats filtra Codex/Claude por proyecto, título o contenido y obtiene la transcripción seleccionada bajo demanda. Errores lista hasta 200 entradas recientes filtradas por la ruta exacta del proyecto.
- Se agregaron `bridge chats`, `bridge conversation` y `bridge errors`. Pruebas cubren mensajes e historial acotados al proyecto y errores de otro proyecto excluidos. CLI smoke: contratos de conversaciones y errores válidos. Suite completa actual: `191 passed, 1 skipped`; `python -m compileall -q src tests` y `git diff --check HEAD` pasan. `QT_QPA_PLATFORM=offscreen timeout 3s /tmp/chxchx-studio-build/chxchx-studio .` mantuvo la app abierta durante el smoke de inicio; timeout 124 esperado.
- CLI smoke en `--dry-run` de trust, start, suspend, resume, stop y switch terminó con código 0, sin ejecutar operaciones reales. `python -m compileall -q src tests`, `git diff --check HEAD` y la suite (`188 passed, 1 skipped`) pasan. El contrato se comprobó con `project_status_payload`: esquema `chxchx.project-status` v1, cuatro proyectos registrados y estado de trust.
- Smoke seguro: CLI `agent start codex --dry-run`, `agent start --all --dry-run` y `agent start codex --new-chat --dry-run` terminaron con código 0; no se iniciaron agentes. No se probó el dry-run de procesos porque este proyecto no configura procesos.
- `README.md` documenta requisitos y comandos de build. Se fijó Qt mínimo 6.4 para ampliar compatibilidad con toolchains de Ubuntu/Fedora.
- Las consultas CLI usadas por el shell se ejecutaron en este proyecto y terminaron con código 0. `git diff --check HEAD` pasa.
- Contrato comprobado por CLI más parseo JSON: `chxchx-tech bridge status .` entrega `schema=chxchx.project-status`, versión 1 y los cinco grupos esperados (`project`, `workspace`, `agents`, `processes`, `resources`). `python -m compileall -q src` pasa; no se ejecutó la suite.
- Build nativo Linux validado con Qt 6.11.2 y Scintilla/Lexilla; Windows/macOS y otras distribuciones dependen de CI.
- La decisión de producto cambió: la interfaz/editor será una aplicación nativa C++20 + Qt 6 Widgets + Scintilla/Lexilla, multiplataforma macOS/Windows/Fedora/Ubuntu y con paridad funcional completa con la TUI. `.ai/EDITOR-PARITY.md` contiene la matriz de funciones y plan de entregas.
- El estado objetivo es un núcleo C++ común a la app, CLI y TUI. Se iniciará con un bridge JSON temporal a servicios Python para conservar paridad; CLI/TUI delegarán por dominios al core nativo al validar cada migración.
- Qt 6.11.2 y CMake/GCC/Ninja están disponibles en este entorno. Para Core5Compat se usaron archivos de desarrollo extraídos bajo `/tmp` por falta de privilegios de instalación.
- CLI y TUI consumen tokens compartidos de color para estados y acentos. Se agregó `editor setup [PATH]` para generar `.ai/sublime/<proyecto>.sublime-project` con exclusiones comunes, `--dry-run`, escritura idempotente y backup al reemplazar una configuración administrada. `--no-open` deja solo el archivo.
- Validación tras esta integración: `python -m compileall src tests`, suite completa (`188 passed, 1 skipped`) y `git diff --check HEAD` limpios.
- Verificación del RAM Governor inicial: `python -m compileall src tests`, suite completa (`184 passed, 1 skipped`) y `git diff --check HEAD` limpios. Todos los módulos de producción quedan bajo 300 líneas.
- RAM Governor inicial: `workspace.resources` ahora incluye `max_agents` (default 2); proyectos nuevos usan aviso RAM 70%, crítico 85% y aviso swap 40%. La TUI calcula cuántos agentes nuevos se abrirían, muestra un diálogo al superar umbrales o presupuesto y deja cancelar o continuar. `agent start`, `run` y `projects switch` hacen el preflight en CLI y requieren `--force` para confirmar. No mata procesos automáticamente.
- El panel Recursos muestra los umbrales y la cuota. El comando `chxchx-tech resources` también imprime esta política. Configs existentes mantienen sus umbrales explícitos; en este repo siguen en 75%/90% y reciben `max_agents = 2` por default al cargar.
- Pruebas nuevas validan configuración, avisos de RAM/cuota y cancelar o confirmar desde el diálogo TUI.
- Se añadió `ui-ux-design`, una guía avanzada de diseño de producto UI/UX; se recomienda en proyectos React, Next.js y React Native y ya forma parte de esos Tech Packs. Catálogo actual: 17 skills.
- La TUI tiene una pestaña principal `Skills` con catálogo buscable y Tech Packs. Muestra total de skills, selección activa, packs detectados, razones de recomendación y composición de cada pack.
- Desde la TUI se puede habilitar/deshabilitar skills, aplicar un pack o todos los packs detectados y sincronizar `.ai/SKILLS.md`/`AGENTS.md`; cada escritura preflighta con `dry-run` y las operaciones existentes hacen backup.
- La vista mantiene dos subpestañas, Catálogo y Tech Packs, para que las acciones sigan accesibles en una terminal pequeña; se puede abrir desde `Ctrl+P` buscando “skills”.
- Validación tras integrar la TUI: `python -m compileall src tests` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (180 passed, 1 skipped); `git diff --check HEAD` limpio.
- Inicio de v0.6: existe `SkillRegistry` con metadatos TOML e instrucciones Markdown, y el CLI expone `skill list/search/info/recommend`.
- Catálogo actual: 17 skills generales y para Python, TypeScript, React/Next, UI/UX, NestJS, .NET, React Native, PostgreSQL, Prisma, Redis y Docker; `recommend` usa `detect_project` para comparar stacks y lenguajes.
- `docs/` sigue presente en esta copia local y está excluida del índice de Git; `.gitignore` evita volver a agregarla.
- Basic Memory sigue apuntando a otro proyecto en esta sesión; no se guardó información allí.
- Validación actual del Skill Registry y Tech Packs: `python -m compileall src tests` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (179 passed, 1 skipped); pruebas CLI cubren catálogo, estado, recomendaciones, detección y aplicación aditiva con dry-run.
- El catálogo creció a 17 skills para stacks detectados y guías generales; el detector reconoce React, Prisma, Redis y Docker además de sus marcadores anteriores.
- `skill enable/disable` guarda selección local por proyecto, `skill sync` genera `.ai/SKILLS.md` con bloque administrado y referencia desde `AGENTS.md`; las mutaciones ofrecen `--dry-run` y backups.
- Tech Packs TOML componen skills; `pack list/detect/info/apply` explica coincidencias y agrega skills de forma aditiva. `skill recommend` expone razones basadas en stack/lenguaje o guía general.
- Se endurecieron tres pruebas asíncronas de la TUI para esperar hasta 5 segundos la finalización de workers en CI; antes usaban ventanas fijas de 200–400 ms.
- Verificación local del ajuste: `python -m compileall src` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- El test de recuperación de procesos parchea el terminador del árbol en Windows y el terminador POSIX en otros sistemas, evitando invocar `taskkill` durante el test.
- Ese test usa ahora un handle simulado y no inicia procesos reales; verifica la recuperación y el despacho al terminador de plataforma sin bloquear el CI de Windows.
- Inicio ofrece acceso directo a Terminal (`T`) y Agentes (`A`).
- La pestaña `Agentes` apila a todo el ancho el encabezado y el panel de uso; debajo acomoda los panes de agentes según `workspace.layout.orientation`.
- En sesiones ya existentes sin pestaña `Agentes`, su layout y panes se crean mientras el cliente Zellij está adjunto; el foco se aplica después de la conexión.
- El panel de uso recibe `--root` sin comillas duplicadas.
- Codex y Claude iniciaron en una prueba real; Claude mostró su confirmación de confianza propia para la carpeta.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.

## Estado del runtime observado

- La sesión guardada `chichan-tech-lead` apareció `EXITED` y su metadata vieja mostraba panes de agentes sin filas de contenido. El inicio del workspace recrea sesiones `EXITED`; para una sesión viva sin pestaña `Agentes`, el flujo nuevo la agrega al adjuntarse.
