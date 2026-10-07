# Handoff

## Studio — editor, tabs y árbol de proyecto

- El árbol ahora abre archivos y despliega carpetas con un clic; el indicador de rama permite contraerlas. Tabs document-mode se eliden por el centro, muestran rutas relativas y usan punto cian al estar modificadas. Los colores CSS se convierten al orden RGB que pide Scintilla, corrigiendo el fondo marrón y los tonos de sintaxis invertidos. La etiqueta queda alineada a la izquierda y la `×` tiene zona fija alineada a la derecha.
- `assets/logo-chxchx-min.png` se usa como icono de la app/ventana y `assets/logo-chxchx.png` se presenta en Marca; ambos van dentro de `branding.qrc` para resolverlos desde recursos Qt.
- Validación: `cmake --build /tmp/chxchx-studio-post-pull --parallel 2` compiló y enlazó; CTest pasó 2/2 y `git diff --check HEAD` está limpio. Pendiente abrir Studio y revisar legibilidad/espaciado en un proyecto con varios archivos y pestañas.

## Terminal Studio — scrollback

- `VtTerminalWidget` guarda hasta 5000 líneas completas de celdas VT/ANSI mediante callbacks de libvterm (`sb_pushline/popline/clear`). La vista histórica mezcla líneas guardadas y pantalla actual; las celdas conservan atributos y color.
- Navegación: rueda en modo terminal normal, `Shift+PageUp/PageDown` por página, `Ctrl+Home` al historial más antiguo y `Ctrl+End` al live bottom. Un badge indica cuántas líneas atrás está la vista; cualquier entrada de teclado vuelve al live bottom. Si la app activa mouse reporting/alternate screen, la rueda se entrega a la app como antes.
- Build macOS reportado y build Linux confirmado: `cmake --build /tmp/chxchx-studio-post-pull --parallel 2`; CTest pasó 2/2 (editor y PTY). Pendiente probar visualmente reflow al resize y scrollback de salida larga. Attach integrado a Zellij sigue fuera de este bloque.

## Terminal de workspace visible en Studio

- Añadido botón de toolbar `Terminal +` (atajo `Ctrl+Shift+T`) y entrada en la paleta. Consulta `bridge status` en modo de solo lectura antes de abrir una pestaña shell integrada en el proyecto; no necesita Zellij. El widget delega el foco al terminal VT para que el teclado llegue a la shell.
- El widget muestra ruta/sesión y permite cerrar la sesión; cerrar la pestaña destruye el widget y detiene el PTY. Attach a Zellij continúa en el emulador externo.
- El build del scrollback también completó al 100% en macOS y Linux. Inicio Qt offscreen mantuvo la aplicación abierta 3 s (timeout esperado); pendiente probar interacción visual.

## Build macOS — definición incompleta y warning

- Se añadió el include `integrations/bridge_client.hpp` a `main_window_agent_sessions.cpp`; ese archivo invoca `BridgeClient::isRunning()` y la forward declaration sola no basta.
- Se eliminó la captura lambda `areaSearch` que no se usaba.
- Confirmación en macOS: `cmake --build build/native --parallel 2` compiló ambos archivos y enlazó `chxchx-studio` al 100%.

## Studio — primera pasada visual

- Tema nativo actualizado a azul noche con acentos cian; estados de foco, hover, selección, pestañas y paneles tienen jerarquía más visible.
- La navegación lateral identifica el proyecto activo y permite filtrar secciones por nombre. La siguiente revisión debe abrir Studio y ajustar contraste/espaciado según el render real; no se ejecutó build en este bloque.

## Terminales integradas — alcance contractual

- El contrato JSON se declara en `src/chxchx_tech_lead/workspace/bridge_contracts.py` y `native/src/integrations/bridge_schemas.hpp`; el stream interactivo PTY/ConPTY es un canal nativo independiente del bridge.
- Studio integra sesiones de agentes y terminal nueva del workspace mediante PTY/ConPTY y libvterm. Attach a Zellij sigue abriendo el emulador externo. El scrollback está implementado; después cubrirlo con pruebas VT.

## Bridge JSON — compatibilidad del consumidor Qt

- Studio comprueba `schema_version` antes de enrutar la respuesta JSON. La versión soportada está centralizada como `BridgeSchemas::Version = 1`; falta de versión, tipo inválido o versión distinta produce un error visible. Los schemas desconocidos también se informan en lugar de mostrar JSON crudo como resultado válido.
- El consumidor sigue leyendo solo los campos que usa, por lo que los campos adicionales permanecen compatibles.
- Validación posterior: `cmake --build build/native --parallel 2` compila Studio al 100% en macOS; `git diff --check` limpio. No se ejecutó la suite de pruebas en esta ronda.

## ChxChx Studio — primer corte nativo

- Estado vigente: Studio integra sesiones de agentes en pestañas mediante PTY/ConPTY y libvterm. Los cambios locales sustituyen el bypass de trust/RAM por `agent preflight`, agregan revisión de presupuesto para sesiones Studio activas y usan `agent_pane_command` tanto para sesiones normales como para chat nuevo; este último conserva el prompt contextual y la instrumentación compartida. Mantener el emulador externo como fallback.
- Validación del cambio: `python -m compileall -q src tests scripts`; suite completa `195 passed, 1 skipped`; `git diff --check` limpio. Qt6 Core5Compat está instalado. Tras recuperar conectividad, el build nativo de Linux compila y CTest pasa 2/2. Tests Python verifican trust denegado, suma de sesiones Studio activas y comandos compartidos de lanzamiento/contexto.
- Qt6 Core5Compat de desarrollo está instalado en Fedora (`qt6-qt5compat-devel-6.11.2`); CMake descargó las dependencias upstream y generó el build local.
- PTY/ConPTY + libvterm están integrados. Trust y RAM pasan por un preflight CLI read-only; las advertencias requieren confirmación explícita y cancelar no crea procesos. Studio obtiene del bridge comandos `agent_pane_command` compartidos, incluido el prompt de chat nuevo. El ADR-0001 local y `.ai/EDITOR-PARITY.md` reflejan la dirección vigente. Siguiente: validar el scrollback nuevo y completar attach integrado a Zellij.
- Pendientes posteriores a la terminal: búsqueda por contenido (la búsqueda por nombre/ruta y los splits ya están implementados), recientes, Git diff, símbolos/LSP, empaquetado, QA visual y métricas por SO. La paridad completa sigue sujeta a `.ai/EDITOR-PARITY.md` y evidencia de CI remoto.
- Bloque nuevo: Configuración, Guía y Marca tienen vistas propias. Configuración conecta `setup`, `init`, `init --minimal`, `install`, `integrate --client all` y `doctor`; las escrituras previsualizan con `--dry-run` y esperan confirmación. `Ctrl+P` permite buscar áreas, abrir/guardar/buscar archivos, refrescar y disparar acciones del panel actual. `Ctrl+F` busca en el documento activo.
- CI agrega builds de Qt 6.8.3 en Ubuntu, Windows y macOS y un build usando paquetes de Fedora. El build Fedora local de Qt 6.11.2 pasa; los runners remotos todavía deben completar para validar las matrices.
- Validación reciente: `cmake --build /tmp/chxchx-studio-build --parallel 2`; `python -m compileall -q src tests scripts`; `uv run --no-sync pytest` (191 passed, 1 skipped); `chxchx-tech setup . --dry-run`; `chxchx-tech doctor .`. El smoke gráfico offscreen continúa abierto durante 3 s (timeout esperado).
- Rama activa `dev`. Añadido `native/` con target CMake/C++20/Qt 6 Widgets y shell de escritorio; la UI inicial trae navegación de áreas, árbol de archivos, pestañas, edición básica y guardado atómico.
- Las vistas Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs, Recursos, Handoff, Memoria, Chats y Errores consumen consultas CLI asíncronas mediante `QProcess`.
- Existe `chxchx-tech bridge status PATH`, contrato JSON versionado (`chxchx.project-status`, versión 1) que entrega metadatos, trust/workspace, agentes, procesos, recursos y umbrales del Governor. Es una lectura y usa `ProcessManager.list(persist=False)` para no persistir cambios al consultar. Recursos agregado usa el contrato separado `chxchx.resources-overview`.
- Agentes y Procesos ya exponen acciones en el panel nativo: agente seleccionado/todos/nuevo chat; proceso seleccionado iniciar/detener. El bridge incluye procesos configurados aunque aún no tengan registro de ejecución. Las mutaciones previsualizan con `--dry-run`, confirman antes de ejecutar y refrescan estado; el RAM Governor evalúa otra vez justo antes del inicio real y exige confirmación explícita para `--force`.
- Workspace ofrece trust, start/resume, suspend y stop con preview y confirmación. Proyectos se lista desde el bridge, permite dar trust al destino, bloquea Switch para el actual/no confiable y cambia de contexto con `--no-attach`; al confirmar el cambio Studio mueve el árbol/cwd al nuevo proyecto. El RAM Governor se vuelve a comprobar y pide confirmación vigente antes de `--force`.
- Skills y Packs consumen el mismo bridge JSON: catálogo y selección, razones de recomendación, composición y detección. Studio habilita/deshabilita skills, aplica packs individuales o detectados y sincroniza contexto con preview `--dry-run` y confirmación.
- Handoff tiene bridge de solo lectura y formulario nativo para editar resumen, pendiente y validación; el guardado conserva el caso de uso existente `workspace handoff` y muestra preview/confirmación. Memoria busca títulos/contenido de `.ai/memory`, permite abrir el detalle y se mantiene de solo lectura; las rutas fuera del proyecto se rechazan.
- Chats y Errores son vistas nativas de solo lectura. `bridge chats` busca sesiones Codex/Claude; al elegir una, `bridge conversation` trae sus mensajes. `bridge errors` limita el listado de caché a este proyecto.
- Validación añadida: suite completa `191 passed, 1 skipped`, `python -m compileall -q src tests`, `git diff --check HEAD`, smoke JSON de chats/errores y build CMake exitoso con Qt 6.11.2. El binario también permaneció abierto durante 3 s bajo `QT_QPA_PLATFORM=offscreen` (timeout 124 esperado).
- Pendiente en ese checkpoint: attach integrado a Zellij, búsqueda por contenido, archivos recientes, Git diff, símbolos, LSP, empaquetado y QA visual/performance; la búsqueda de rutas y los grupos divididos se completaron después.
- README actualizado con requisitos y comandos para compilar. `git diff --check HEAD` pasa.
- Build Linux confirmado. El primer intento encontró `QFileSystemModel::isFile`, API inexistente; se corrigió usando `!isDir(index)` y el build pasó.
- Basic Memory no se actualizó porque la herramienta disponible en esta sesión estaba asociada a otro proyecto; este checkpoint queda local.

## RAM Governor (v0)

- `workspace.resources.max_agents` agrega una cuota suave por proyecto, default 2. Proyectos nuevos usan RAM warning 70%, crítico 85%, swap warning 40%; configuraciones que declaran sus propios umbrales los conservan.
- La TUI revisa la carga estimada antes de iniciar agentes por las acciones de agente individual, chat nuevo, inicio de agentes y preparación/attach de panes. Si hay presión de RAM/swap o se supera la cuota, pide confirmar o cancelar.
- El CLI también preflighta `agent start`, `run` y `projects switch`; exige `--force` para continuar bajo avisos, mientras `--dry-run` los muestra sin iniciar.
- Recursos y `chxchx-tech resources` muestran política y umbrales. No hay suspensión ni terminación automática de procesos.
- Pruebas de unidad y TUI cubren límites, conteo previsto, confirmación y cancelación.
- Validación: `python -m compileall src tests`, suite completa (`184 passed, 1 skipped`) y `git diff --check HEAD` limpios.
- Próximo paso posible: evaluar cuotas de RAM disponible por proyecto y un panel histórico, manteniendo la detención de procesos como acción manual.

## Plan anterior de modos CLI, TUI y editor (superado por la decisión nativa)

- El roadmap fija un contrato común sobre servicios de workspace, con CLI, TUI y editor como interfaces independientes. Configuración y estado de proyecto son compartidos; foco, paneles y estado de pantalla son propios de cada interfaz.
- La CLI seguirá siendo utilizable sin abrir la TUI o Sublime y conserva los preflight de RAM/cuota.
- Próximo alcance para editor: extender el adapter existente de Sublime con generación idempotente del proyecto, exclusiones según stack y apertura desde ChxChx. Después se puede agregar un paquete pequeño de comandos Sublime que invoque la CLI.
- La interacción desde una interfaz no debe iniciar, detener ni duplicar sesiones que otra interfaz ya administra.
- Evaluar editor propio más adelante, con alcance y costos de mantenimiento definidos según el uso real.

## Primera integración visual y Sublime

- `ui.theme` centraliza tonos compartidos; la CLI usa colores semánticos acordes con los estados de la TUI y las hojas CSS de TUI/paleta resuelven sus colores mediante los mismos tokens.
- `chxchx-tech editor setup [PATH]` crea `.ai/sublime/<proyecto>.sublime-project` y lo abre. `--dry-run` no escribe ni ejecuta; `--no-open` genera el archivo sin abrir Sublime.
- Los patrones excluyen carpetas usuales de dependencias/build/cache y archivos temporales. Los proyectos son locales a `.ai/`.
- El generador es idempotente, conserva backup antes de cambiar un archivo marcado como administrado y se niega a sobrescribir un archivo no administrado. No toca preferencias globales ni impone un color scheme al usuario.
- Sublime no permite controlar algunas preferencias visuales de interfaz a nivel de proyecto; se conserva el tema elegido por el usuario. La consistencia inmediata cubre paleta/estados de CLI y TUI, y el flujo de uso del editor.
- Validación: `python -m compileall src tests` y `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (188 passed, 1 skipped); `git diff --check HEAD` limpio.

## Requisito de paridad completa en el editor

- El usuario requiere que todas las áreas y operaciones de la TUI estén disponibles en la interfaz de editor, con integración visual cuidada y sin añadir peso innecesario.
- Matriz completa: `.ai/EDITOR-PARITY.md`. Incluye Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs, Recursos, Handoff, Memoria, Chats, Errores, Guía/Marca, Configuración y Paleta.
- Nueva dirección aceptada por el usuario: aplicación de escritorio C++ nativa, multiplataforma, con editor embebido y velocidad como criterio principal. No depender de Sublime ni Kate.
- Stack propuesto: C++20 + Qt 6 Widgets para UI KDE-inspired multiplataforma; Scintilla/Lexilla para el editor C++ liviano y permisos de licencia permisivos. No usar Qt WebEngine/Chromium.
- Estado objetivo: un núcleo de dominio C++ compartido por app, CLI y TUI. El bridge JSON a Python es transición temporal por paridad; portar por dominios y cambiar autoridad solo después de pruebas comparativas. La ventana usa QProcess/consultas asíncronas.
- Cobertura exigida: cada fila de `.ai/EDITOR-PARITY.md`; paquetes para Fedora, Ubuntu, macOS y Windows; CI por plataforma; medir memoria/startup/latencia.
- Este entorno Fedora ahora tiene Qt 6.11.2, CMake 4.3, GCC 16 y Ninja; el target nativo configura y compila correctamente.

## Nueva skill UI/UX

- Se agregó `ui-ux-design` con guía de jerarquía, sistema visual, accesibilidad, estados, responsive y verificación.
- Se recomienda automáticamente para React, Next.js y React Native y se incluye en `react-web`, `nextjs-app` y `react-native-mobile`.
- El catálogo cuenta con 17 skills; README y pruebas CLI/TUI reflejan el nuevo total.

## Ampliación de TUI: Skills

- La pestaña principal `Skills` permite buscar el catálogo, consultar detalles/recomendaciones y ver los 11 Tech Packs con composición y coincidencias del proyecto.
- Catálogo y Tech Packs están separados en subpestañas para que controles y listas sean accesibles con terminales cortas.
- Se pueden habilitar/deshabilitar skills, aplicar un pack individual o la unión de los detectados y sincronizar el contexto; el flujo preflighta mutaciones con `dry-run` y conserva backups mediante el servicio existente.
- `Ctrl+P` incluye “Ver catálogo de skills y Tech Packs”.
- README y arquitectura local explican la vista nueva.
- Validación: `python -m compileall src tests`, `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run --no-sync pytest` (180 passed, 1 skipped) y `git diff --check HEAD` limpios.
- Próximo paso sugerido: probar visualmente `chxchx-tech tui .` en el terminal habitual, especialmente el cambio Catálogo/Tech Packs y el flujo de habilitar y sincronizar.

## Inicio de Skill Engine (v0.6)

- Rama `dev`; los documentos versionados de `docs/` se retiraron del índice, sin borrar la copia local. `/docs/` quedó en `.gitignore`.
- Añadidos el módulo `skills.registry` (descubrimiento, búsqueda, metadatos e instrucciones) y el subcomando `skill` del CLI (`list`, `search`, `info`, `recommend`).
- La biblioteca contiene 17 skills generales y para Python, TypeScript, React/Next, UI/UX, NestJS, .NET, React Native, PostgreSQL, Prisma, Redis y Docker. El detector recomienda según stacks y lenguajes; las guías de API, arquitectura, seguridad, testing y code review son generales.
- `skill enable/disable <nombre> [proyecto]` mantiene `.ai/chxchx-skills.toml`; `skill sync [proyecto]` publica solo las instrucciones seleccionadas en `.ai/SKILLS.md` y enlaza ese contexto desde `AGENTS.md`.
- `--dry-run` muestra operaciones sin escrituras; cambios al config y archivos administrados hacen backup antes de escribir. La actualización de bloques preserva contenido alrededor de ellos.
- Añadidos conteos visibles al catálogo, `skill status` para selección por proyecto y `pack apply-detected` para aplicar la unión sin indicar nombres. README aclara que el catálogo viene con ChxChx y que las skills son instrucciones, no paquetes ejecutables.
- Añadidas pruebas de búsqueda, carga de instrucciones, detección de stack, enable/dry-run, sincronización idempotente preservando contenido local, aplicación aditiva de packs, listados y conteos. `python -m compileall src tests` y la suite pasan (179 passed, 1 skipped).
- `uv run pytest` sin `--no-sync` no pudo descargar setuptools porque este entorno no resuelve PyPI; la suite pasó con `uv run --no-sync pytest` usando el entorno existente.
- ADR local 0002 registra el formato y las reglas de selección; README incluye el flujo de comandos.
- Próximo paso: medir el costo de contexto sincronizado y definir un flujo seguro de importación con procedencia/licencia antes de añadir fuentes externas.

## Último cambio

- El test de recuperación del ProcessManager usa un handle simulado. Cubre persistencia, recuperación y selección del terminador sin iniciar un subprocess de larga duración, eliminando la espera de `taskkill` en Windows.
- El test de recuperación del ProcessManager ahora parchea el terminador correspondiente a la plataforma. En Windows evitaba un bloqueo al no ejecutar `taskkill` sobre el proceso largo de prueba.
- Los tests de ciclo de vida de la TUI esperan la finalización de acciones, dashboard y consola con un límite de 5 segundos, en vez de usar sleeps de 200–400 ms. Esto evita cerrar la app de prueba antes de que Textual procese el worker, como ocurrió en CI de Windows/Python 3.11 y 3.12.
- Corregido el eje del layout `Agentes`: encabezado y panel de uso ocupan el ancho; la orientación solo organiza los agentes.
- `agents_tab_layout` ahora genera el formato pane-only que espera `zellij action new-tab --layout`, y el adapter pasa el nombre con `--name`.
- La pestaña y los panes se crean desde el worker de attach después de que el cliente conecte. El worker repite el foco por un intervalo corto porque una acción antes del attach no selecciona la pestaña para el cliente nuevo.
- `prepare_agents` ya no crea panes dinámicos dentro de la última pestaña restaurada. Al abrir un agente, el attach enfoca la pestaña y luego el pane solicitado.
- Corregido `--root` del panel de uso. README actualizado sobre el layout y el diálogo de confianza propio de Codex/Claude.
- Se conservan el acceso directo `A` y el arreglo de foco por ID para panes de agentes.

## Pendiente

- El usuario debe salir de la TUI abierta, ejecutar `chxchx-tech tui .` y pulsar `A`.
- Si ya tiene una pestaña `Agentes` creada con el layout anterior, recrear esa sesión una vez con `chxchx-tech run . --recreate`. Claude Code puede pedir que se confirme su propia confianza de carpeta.

## Validación

- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/workspace/test_process_manager.py` (13 passed).
- `python -m compileall src tests scripts` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/workspace/test_process_manager.py` (13 passed).
- `python -m compileall src tests scripts` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `python -m compileall src` (correcto).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest tests/tui/test_lifecycle.py` (16 passed).
- `UV_CACHE_DIR=/tmp/chichan-tech-lead-uv-cache uv run pytest` (165 passed, 1 skipped).
- `UV_CACHE_DIR=/tmp/chxchx-uv-cache uv run --no-sync python -m compileall src`
- `UV_CACHE_DIR=/tmp/chxchx-uv-cache uv run --no-sync pytest` (165 passed, 1 skipped).
- Zellij real, sesión existente con Terminales: el attach agregó `Agentes`, mostró 4 panes (header 2×164, uso 8×164, agentes 27×82), imprimió contenido en ambos y enfocó Codex.
- Inicio real de las CLIs configuradas: Codex mostró su TUI y Claude pidió confirmar la confianza de la carpeta. Las sesiones de smoke test se eliminaron.
- `git diff --check`.

## Basic Memory

- `list_memory_projects` encontró `chichan-tech-lead-e45505`, pero `recent_activity` ignoró el `project_id` explícito y respondió desde `bokana-fa6b1d`. No se escribió en ese proyecto para evitar mezclar memoria de otro repositorio; los checkpoints quedan en estos archivos locales.

## Studio: navegación del árbol

- En `native/src/main_window_layout.cpp`, el árbol de proyecto abre archivos con un clic y ya no expande carpetas con doble clic.
- `MainWindow::openTreeFile` alterna el estado expandido de la carpeta al hacer clic en su fila; el tooltip explica la interacción.
- Validación Linux: `cmake --build /tmp/chxchx-studio-post-pull --parallel 2`, CTest (2/2) y `git diff --check` pasan.
- Para revisión visual, cerrar la instancia abierta y ejecutar `/tmp/chxchx-studio-post-pull/chxchx-studio "$PWD"` desde la raíz del repo.

## Studio: menús y atajos

- Acciones de archivo, edición, vista, navegación, herramientas y ayuda quedaron agrupadas en menús superiores; las acciones más usadas siguen en la barra de acceso rápido.
- `Edición → Configurar atajos` permite editar combinaciones, rechaza duplicados, guarda con `QSettings` y restablece los valores iniciales.
- Build Linux y CTest (2/2) pasan; `git diff --check` limpio.

## Studio: iconos de navegación

- La barra de acceso rápido muestra iconos sin etiquetas; los tooltips incluyen acción y atajo vigente. Las 15 secciones laterales tienen iconos semánticos.
- Se consultan iconos del tema del escritorio y se usa `QStyle` de Qt como fallback multiplataforma.
- Build Linux, CTest (2/2), inicio offscreen y `git diff --check` pasan.

## Studio: explorador y búsqueda indexada

- El árbol conserva iconos nativos de `QFileSystemModel` a tamaño 18×18 y añade un filtro por nombre que mantiene visibles los directorios ante coincidencias descendientes.
- `Navegar → Buscar archivo del proyecto` (`Ctrl+Shift+F`) abre resultados globales por nombre/ruta relativa. `ProjectFileIndex` escanea en segundo plano, omite `.git`, `node_modules`, `build`, `dist`, caches y entornos virtuales; la búsqueda se limita a 250 resultados, el índice se limita a 100.000 archivos y permite reconstruirlo.
- Build Linux, CTest (2/2), inicio offscreen de 3 segundos y `git diff --check` pasan. Pendiente: revisión visual e interacción en escritorio real.

## Studio: paneles y estado de secciones

- La barra de acceso rápido permite mostrar/ocultar Explorador, Secciones/acciones y Actividad con el estado checkable sincronizado con `Ver`.
- Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills, Tech Packs y Recursos muestran sus estados en una página central; Actividad conserva el registro de operaciones.
- Skills muestra el estado habilitada/no habilitada y recomendación. Agentes muestra disponibilidad y actualiza “En uso en Studio” al abrir/cerrar sesiones.
- Build Linux, CTest (2/2), inicio offscreen de 3 segundos y `git diff --check` pasan. Falta QA visual en el escritorio del usuario.

## Studio: conservar archivos y terminales al navegar

- Las vistas de sección están dentro de una pestaña fija del mismo `QTabWidget` que documentos, terminales y agentes. Seleccionar una sección solo cambia el contenido de esa pestaña y mantiene intactos los tabs de trabajo.
- La pestaña de sección actualiza su icono y título según la navegación lateral, no muestra botón de cierre y `closeEditorTab` la protege.
- Build Linux (`/tmp/chxchx-studio-post-pull`) y CTest (2/2) pasan; falta QA visual/interactivo en escritorio.

## Studio: suavizar texto claro del editor

- El texto base usa `#B9C6D8` en lugar de `#DCE8F7` y los tokens azul claro usan `#98B5CC` en lugar de `#A5D6FF`; números de línea y fondo no cambiaron.
- `cmake --build /tmp/chxchx-studio-post-pull --parallel 2` completó. Falta revisión visual del usuario.

## Studio: fondo de números y word wrap

- El gutter se fuerza a azul marino `#122941` y números `#7896B5` después de configurar el lexer, evitando el fondo gris heredado.
- El editor abre con word wrap; `Ver → Ajuste de línea` y `Alt+Z` lo alternan. Al activarlo se oculta el scroll horizontal; al desactivarlo vuelve a mostrarse. La elección persiste en `QSettings`.
- Build Linux completado. Falta QA visual/interactivo del usuario.

## Studio: compactar pestañas

- La pestaña de secciones queda como icono de casa sin texto. Las pestañas de editor/terminal/agentes se compactaron (alto 28 px, ancho máximo 190 px, padding y botón de cierre más pequeños).
- Build Linux y `git diff --check` pasan. Queda revisión visual del usuario.

## Studio: títulos breves de pestaña

- Inicio ahora tiene ancho mínimo de 44 px. Los archivos muestran solo nombre+extensión y conservan la ruta completa en tooltip.
- Las terminales integradas usan nombres consecutivos `Terminal #N`.
- Build Linux y `git diff --check` pasan. Pendiente confirmar visualmente que el ancho es cómodo.

## Studio: split y grupos de pestañas

- Se agregó un divisor redimensionable con dos grupos. Clic derecho en cualquier pestaña de archivo/terminal/agente ofrece dividir a la derecha, dividir abajo y mover al otro grupo.
- “Cerrar grupo dividido” fusiona sus pestañas al grupo principal; no termina los procesos de terminal ni las sesiones de agentes.
- Menús, guardado, cierre, detección de archivo actual y ajuste de línea consideran ambos grupos. Build Linux y `git diff --check` pasan; falta revisión interactiva del usuario.

## Studio: edición general y atajos

- Se añadieron Deshacer/Rehacer, Cortar/Copiar/Pegar y Seleccionar todo al menú Edición; cada acción tiene shortcut ID configurable y usa la secuencia estándar de Qt de cada plataforma.
- Los comandos actúan sobre el editor o campo enfocado. Terminal mantiene Ctrl+C para interrumpir y Ctrl+V pega el portapapeles mediante bracketed paste.
- Build Linux, inicio offscreen de 3 segundos y `git diff --check` pasan; falta QA manual en campos, editor y terminal.

## Studio: recorrido guiado para preparar un proyecto

- La sección Guía ahora muestra una ruta de cuatro pasos: Init/configuración, selección y sincronización de skills, autorización e inicio del workspace, y apertura de una nueva sesión con contexto.
- Los botones de cada paso llevan a la sección correspondiente. También se incluyeron indicaciones rápidas sobre el explorador, terminal, divisiones y atajos.
- Build Linux en `/tmp/chxchx-studio-post-pull`, inicio offscreen por 3 segundos y `git diff --check` completados. Próximo paso: revisar visualmente la ruta y probar sus cuatro botones en Studio.

## Cierre funcional para empezar a usar Studio

- `.ai/ROADMAP.md` y `.ai/EDITOR-PARITY.md` ahora reflejan el producto real: Studio nativo, búsqueda indexada de rutas y grupos divididos implementados. La búsqueda de contenido, Zellij embebido, recientes, Git diff/símbolos/LSP, métricas y paquetes quedan en pendientes por prioridad.
- Siguiente paso concreto: QA manual en Fedora con un proyecto confiable usando la Guía: Init/previsualización, trust, skills y sync, abrir/editar/guardar, shell integrada, nueva sesión con contexto, confirmar preflight RAM/trust y cerrar Studio verificando procesos.
- Para abrir el checkout: `./scripts/dev-studio.sh /ruta/al/proyecto`; usa el `.venv` local si está creado. En ejecución directa, configurar `CHXCHX_TECH_CLI` o PATH.
- Los cambios de Studio y la limpieza de documentación quedaron en dos commits de `dev`. `docs/` se conserva localmente (18 archivos) y está ignorada por Git; ya no se requiere para entender el repo desde otro equipo. Esta revisión documental no ejecutó pruebas; el build Linux y CTest 2/2 anteriores siguen siendo la última verificación de código registrada.

## Studio: acciones rápidas en Inicio

- Inicio ahora ofrece acciones centrales de trust/workspace, terminal, Init, sync de skills, Doctor y agentes (incluye sesión nueva y abrir todos), sin exigir el panel lateral.
- Cada botón llama el flujo de preview/confirmación existente; iniciar agentes conserva el RAM preflight. El build Linux pasó en `/tmp/chxchx-studio-post-pull`.
- El usuario ya revisó visualmente Inicio y le gustó el panel. Verificación local: `cmake --build /tmp/chxchx-studio-post-pull --parallel 2` pasó; CTest 2/2 pasó (`native-editor-smoke`, `native-pty-smoke`); `python -m compileall src` pasó; pytest 195 passed, 1 skipped. QA no ejecutó acciones mutantes de Init/workspace; usan preview y confirmación al utilizarlas.
- Este bloque está listo para commit en `dev`. `AGENTS.md` y `.codex/` tienen cambios locales del usuario y deben permanecer fuera del commit.

## Studio — smoke automatizado de VT/scrollback

- Se añadió `native-vt-terminal-smoke` con `QT_QPA_PLATFORM=offscreen`. Verifica ANSI coloreado y que Ctrl+Home, Shift+PageDown y Ctrl+End cambien/restauren lo que pinta el widget ante salida larga.
- Validación Linux: build en `/tmp/chxchx-studio-post-pull`, CTest 3/3 (editor, PTY, VT), `git diff --check` limpios.
- Sigue pendiente cobertura de resize/reflow, sesiones interactivas largas y attach integrado a Zellij. Las notas locales `docs/` están ignoradas por Git; el backlog vigente queda resumido en `.ai/ROADMAP.md` y `.ai/EDITOR-PARITY.md`.
