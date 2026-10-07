# Current State

## Estado actual

- Editor y navegación visualmente refinados: colores Scintilla en el orden RGB correcto; pestañas alinean el título a la izquierda y reservan una zona fija para la `×`; un clic abre archivos y alterna expandir/contraer carpetas. Build Linux y CTest pasan; falta revisión visual real.
- Acciones principales agrupadas en menús estándar de Studio y compartidas con la barra de acceso rápido. Los atajos se editan en `Edición → Configurar atajos`, se validan contra duplicados y persisten con `QSettings`.
- La barra de acceso rápido usa iconos con tooltips y atajos; la lista lateral asigna iconos temáticos a sus 15 secciones. Los iconos usan el tema del sistema con fallback Qt.
- El explorador de archivos muestra iconos de `QFileSystemModel`, y su filtro lateral busca nombres en el árbol. `Ctrl+Shift+F` abre búsqueda global de archivos por nombre/ruta sobre índice asíncrono; excluye directorios generados comunes y permite reconstruir el índice.
- La barra de acceso rápido controla la visibilidad de Explorador, Secciones/acciones y Actividad con acciones checkables de los dock widgets. Las secciones operativas presentan estados en el centro; Skills marca habilitadas/recomendadas y Agentes disponibles/en uso en Studio.
- Los logos de `assets/` se empaquetan en recursos Qt: variante mínima como icono de aplicación/ventana y logo completo en la vista Marca.
- Scrollback integrado en `VtTerminalWidget`: conserva hasta 5000 líneas con celdas/estilos ANSI, rueda para navegar, `Shift+PageUp/PageDown` por página y `Ctrl+Home/End` para inicio/final. Muestra badge al revisar historial y vuelve al final cuando se escribe. Conserva rueda como mouse input cuando la app activa modo mouse o alternate screen. `cmake --build build/native --parallel 2` pasó; falta prueba visual.
- Terminal visible añadida a Studio: botón `Terminal +` y `Ctrl+Shift+T` abren el shell del proyecto como pestaña integrada con PTY/ConPTY y libvterm, previa a una consulta bridge de solo lectura que no depende de Zellij. Attach a Zellij sigue externo. Build macOS reportado y build Linux local pasan; falta comprobación visual.
- Corregido error de compilación reportado en macOS: `main_window_agent_sessions.cpp` incluye la definición de `BridgeClient` que necesita para consultar `isRunning()`. Quitada captura lambda no usada en el filtro de secciones. `cmake --build build/native --parallel 2` completó y enlazó `chxchx-studio`.
- Primera pasada de modernización visual de Studio: paleta azul noche/cian, controles con estados hover/focus/selección, separadores y tabs más claros; navegación lateral ahora muestra proyecto y filtra secciones al escribir. Pendiente revisar visualmente y seguir simplificando tareas frecuentes.
- El bridge JSON está descrito por `src/chxchx_tech_lead/workspace/bridge_contracts.py`; las terminales integradas son una capacidad nativa separada. Agentes y terminal nueva de workspace usan PTY/ConPTY; attach a Zellij sigue externo. El siguiente alcance es attach embebido a Zellij.
- Studio ahora valida `schema_version` del bridge antes de procesar un payload y muestra un mensaje legible ante versiones incompatibles, versión ausente/inválida o un nombre de schema desconocido. La versión aceptada por Qt se declara como `BridgeSchemas::Version = 1`; los campos JSON adicionales siguen siendo ignorados por los accesos selectivos existentes.
- Transporte PTY/ConPTY añadido a `native/src/integrations/pty_session.*`: shell o proceso por argv/cwd, entrada, salida asíncrona, resize y cierre. POSIX usa `forkpty` (Linux/macOS); Windows usa ConPTY, mínimo Windows 10.
- `native-pty-smoke` comprueba un shell interactivo: Studio envía una línea al proceso y cambia la geometría; en Linux también valida que `stty size` devuelve el nuevo tamaño. CMake compila y CTest pasa 2/2 en Fedora. El backend ConPTY debe quedar validado por el runner de Windows CI.
- Commit `a5c0753` integra pestañas de agentes con PTY/ConPTY y libvterm. El commit posterior añade `agent preflight` para trust/RAM agregado y pasa el arranque embebido por `agent_pane_command`, incluyendo instrumentación y el prompt contextual de sesión nueva. Python: 195 passed, 1 skipped. El build Linux actual compila y enlaza Studio.
- Cierre del bloque actual: Recursos de Studio consume `bridge resources PATH`, con muestra global del sistema y resumen de procesos gestionados para cada proyecto registrado. El endpoint es read-only y una prueba verifica que conserva el archivo de configuración.
- El editor usa Scintilla/Lexilla fijados a commits upstream; tiene UTF-8, guardado atómico, lexer por extensión y búsqueda en archivo. `workspace terminal` abre una pestaña integrada; attach a Zellij y `agent attach` mantienen launcher externo, con preview/confirmación y preflight de RAM para agentes.
- Validación actual: `python -m compileall -q src tests scripts`; suite Python (`195 passed, 1 skipped`); `git diff --check`; `cmake --build /tmp/chxchx-studio-post-pull --parallel 2`; CTest (`2/2`); inicio Qt offscreen durante 3 s (timeout 124 esperado). Qt6 Core5Compat está instalado; CMake descargó Scintilla/Lexilla/libvterm y compiló el proyecto tras recuperar conectividad.
- Requisito local: el build estándar usa Qt6 Core5Compat de desarrollo; ya está instalado en este Fedora. CI instala el módulo desde sus runners.
- Pendiente inmediato: QA visual/interactivo del flujo Studio en un proyecto real y pruebas automatizadas específicas para pantalla ANSI/VT/scrollback; CTest cubre editor y PTY, no el render VT. La búsqueda indexada de rutas y grupos divididos ya están implementados. Después, según prioridad: attach integrado a Zellij, búsqueda de contenido, recientes, Git diff, símbolos/LSP, instaladores y métricas multiplataforma.
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

## Studio: conservar pestañas de trabajo al cambiar de sección

- Las vistas de sección comparten una pestaña fija con documentos, terminales y agentes. Navegar entre secciones solo cambia el contenido de esa pestaña y conserva abiertas las demás.
- La pestaña fija refleja el nombre/icono de la sección activa, no muestra botón de cierre y está protegida en `closeEditorTab`.
- Build Linux y CTest (2/2) pasan; queda QA visual/interactivo en escritorio.

## Studio: contraste de texto del editor

- El texto base del editor bajó de blanco azulado a un gris azulado suave (`#B9C6D8`); los tokens de sintaxis azul claro también se atenuaron (`#98B5CC`). Se mantienen intactos el fondo, la selección y el color de números de línea.
- Build Linux completado; pendiente revisar visualmente el contraste en escritorio.

## Studio: margen y ajuste de línea

- El margen de números usa un fondo azul marino `#122941` y primer plano `#7896B5`; se reaplican tras la inicialización del lexer para que no vuelva al gris predeterminado.
- El ajuste de línea está activo de forma predeterminada, oculta la barra horizontal y se alterna desde `Ver → Ajuste de línea` o `Alt+Z`. La preferencia queda guardada en `QSettings` y aplica a documentos abiertos y nuevos.
- Build Linux completado; falta revisión visual e interacción con un archivo largo.

## Studio: pestañas compactas

- La pestaña de secciones queda sin texto y conserva solo el icono de Inicio; su tooltip indica que abre las secciones del proyecto.
- Los tabs de archivos, terminales y agentes reducen ancho máximo/mínimo, altura, padding, margen y tamaño de cierre manteniendo el título legible.
- Build Linux y `git diff --check` pasan; falta validar el tamaño visual en escritorio.

## Studio: nombres de pestañas

- La pestaña Inicio usa mínimo de 44 px y el icono de casa a 14 px. Las pestañas de código muestran el basename con extensión (por ejemplo, `main_window.cpp`); el tooltip mantiene la ruta completa.
- Las terminales integradas se nombran `Terminal #1`, `Terminal #2`, etc. durante la sesión de Studio.
- Build Linux y `git diff --check` pasan; falta QA visual del usuario.

## Studio: grupos de pestañas divididos

- El centro usa un `QSplitter` con dos grupos de pestañas. Clic derecho sobre una pestaña permite dividirla a la derecha o abajo y moverla entre grupos; sirve para archivos, terminales y sesiones de agente.
- Cerrar la división devuelve sus pestañas al grupo principal sin cerrar procesos o sesiones. Inicio permanece fijo en el grupo principal; las acciones de archivo y ajuste de línea operan sobre el grupo activo.
- La Guía rápida describe el menú contextual. Build Linux y `git diff --check` pasan; pendiente probar las interacciones de split en escritorio.

## Studio: acciones de edición y atajos

- `Edición` ofrece Deshacer, Rehacer, Cortar, Copiar, Pegar y Seleccionar todo con atajos estándar de Qt; quedan editables desde Configurar atajos.
- Las acciones se despachan al editor Scintilla, campos QLineEdit, paneles QPlainTextEdit/QTextEdit o terminal con el foco. En terminal se conserva Ctrl+C como interrupción; Pegar envía el portapapeles como texto.
- La Guía rápida explica estas acciones. Build Linux, inicio offscreen y `git diff --check` pasan.

## Studio: recorrido guiado para preparar un proyecto

- La sección Guía presenta cuatro pasos en orden: preparar el proyecto con Init, elegir y sincronizar skills, autorizar/iniciar el workspace y abrir una nueva sesión de agente con contexto.
- Cada tarjeta tiene un botón que navega directamente a Configuración, Skills, Proyecto o Agentes. La guía también resume el uso diario del explorador, Terminal +, divisiones de pestañas y configuración de atajos.
- Validación: build Linux en `/tmp/chxchx-studio-post-pull`, inicio offscreen por 3 segundos y `git diff --check` pasan. Falta revisión visual e interacción en escritorio.

## Revisión para empezar a usar Studio

- Roadmap y matriz de paridad actualizados: el plan anterior aún describía Sublime como editor objetivo y marcaba la búsqueda de rutas y los splits como pendientes. Ahora distingue funcionalidades listas, bloqueos para comenzar a usarlo y mejoras de v1.0.
- El cierre mínimo es QA interactivo en un proyecto confiable: Init/trust, skills, edición/guardado, terminal, agente/contexto, RAM y cierre limpio. Los builds/smokes por sistema requieren evidencia real; que exista un job CI no demuestra que pasó.
- Cambios de Studio y limpieza documental quedaron comprometidos en dos commits de `dev`. `docs/` permanece con 18 archivos en este checkout y está ignorada; otros clones no la recibirán. No se ejecutaron pruebas en la revisión documental; el build/CTest de Studio constan en el handoff anterior.

## Studio: acciones rápidas centrales en Inicio

- Inicio incorpora botones centrales para trust, iniciar/reanudar, suspender y detener workspace; terminal; Init completo, sincronización de skills y Doctor; selector de agente con abrir/nueva sesión/abrir todos.
- Los comandos reutilizan los previews y confirmaciones existentes; agentes conservan preflight de trust/RAM. El panel lateral sigue disponible, pero no es requisito para estas acciones frecuentes.
- El usuario revisó el panel en Studio y confirmó que la presentación le gusta. Compilación Linux y CTest 2/2 pasan; `python -m compileall src` pasa y pytest reporta 195 passed, 1 skipped. No se ejecutaron mutaciones de Init/workspace durante QA.
