# Handoff

## ChxChx Studio — primer corte nativo

- Estado vigente: Studio integra sesiones de agentes en pestañas mediante PTY/ConPTY y libvterm. Los cambios locales sustituyen el bypass de trust/RAM por `agent preflight`, agregan revisión de presupuesto para sesiones Studio activas y usan `agent_pane_command` tanto para sesiones normales como para chat nuevo; este último conserva el prompt contextual y la instrumentación compartida. Mantener el emulador externo como fallback.
- Validación del cambio: `python -m compileall -q src tests scripts`; suite completa `195 passed, 1 skipped`; `git diff --check` limpio. Qt6 Core5Compat está instalado. El build nativo queda pendiente: CMake intentó descargar Scintilla desde GitHub y la resolución DNS falló. Tests verifican trust denegado, suma de sesiones Studio activas y comandos compartidos de lanzamiento/contexto.
- Qt6 Core5Compat de desarrollo ya quedó instalado en Fedora (`qt6-qt5compat-devel-6.11.2`). La configuración CMake detecta Qt, pero no completa FetchContent porque falla la resolución DNS de `github.com` al descargar Scintilla.
- PTY/ConPTY + libvterm están integrados. Trust y RAM pasan por un preflight CLI read-only; las advertencias requieren confirmación explícita y cancelar no crea procesos. Studio ahora obtiene del bridge los comandos `agent_pane_command` compartidos, incluido el prompt de chat nuevo. El ADR-0001 local y `.ai/EDITOR-PARITY.md` ya reflejan la dirección vigente. Siguiente: implementar scrollback acotado/navegable y pruebas VT; compilar al recuperar conectividad con GitHub.
- Después de terminal: búsqueda global, recientes, splits, Git diff, símbolos/LSP, empaquetado, QA visual y métricas por SO. No declarar paridad total hasta cubrir `.ai/EDITOR-PARITY.md` y ejecutar CI remoto.
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
- Pendiente: terminal interactiva integrada, búsqueda global, archivos recientes, splits, Git diff, símbolos, LSP, empaquetado instalable y QA visual/performance en cada SO; los builds multiplataforma local no se pueden certificar desde Fedora.
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
