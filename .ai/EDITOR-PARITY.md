# Aplicación nativa y paridad con TUI

## Objetivo

La aplicación de escritorio debe ofrecer edición de código y todas las vistas y acciones de la TUI, con la misma configuración, servicios de workspace, límites de recursos, permisos, memoria y resultados. La disposición puede adaptarse a escritorio, pero ninguna capacidad de la TUI debe quedar inaccesible.

La interfaz debe sentirse ligera y ordenada, cercana a la estética limpia de KDE: navegación consistente, jerarquía clara, acentos compartidos y estados visibles. Abrir o cerrar paneles no inicia, duplica ni detiene procesos.

## Inventario de paridad

| Área actual de TUI | Capacidades que deben estar en el editor |
| --- | --- |
| Inicio | Resumen de proyecto, stack, estado/trust, procesos, recursos y accesos rápidos a terminales/agentes |
| Proyecto | Procesos configurados, salida/estado, iniciar/detener, trust, terminal Zellij y nueva terminal |
| Agentes | Disponibilidad, sesión/pane/preset, iniciar agente, nuevo chat, iniciar todos y adjuntar |
| Procesos | Lista, estado, PID, puerto, RAM/CPU y controles de inicio/parada |
| Proyectos | Lista de registrados, proyecto actual, cambiar proyecto y refrescar |
| Skills | Catálogo, búsqueda, detalle/recomendaciones, habilitar/deshabilitar y sincronizar |
| Tech Packs | Detección/razones, composición, aplicar pack o unión detectada, sincronizar |
| Recursos | RAM/swap/CPU del sistema, recursos de procesos del proyecto y vista agregada por proyecto |
| Handoff | Leer estado, editar resumen/pendientes/validación y guardar mediante servicio compartido |
| Memoria | Buscar y leer notas del proyecto; mantener solo lectura |
| Chats | Buscar y leer historial local de Codex/Claude; mantener solo lectura |
| Errores | Lista y detalle de errores recientes del proyecto |
| Guía y marca | Guía operativa, navegación y marca CHXCHX |
| Configuración | Preview, init completo/mínimo, herramientas, MCP, preparación completa y doctor |
| Paleta | Buscar y ejecutar cada acción del editor sin depender de atajos memorizados |

## Dirección técnica

### Aplicación: C++20 y Qt 6 Widgets

- Shell de escritorio, paneles y navegación escritos en C++20 con Qt 6 Widgets y CMake; Qt soporta Windows, macOS y Linux y adapta controles a la plataforma.
- Editor de texto basado en ScintillaQt/Lexilla, evitando Chromium/WebEngine y bindings que no sean necesarios. El núcleo de edición y análisis léxico queda en C++.
- UI KDE limpia y de bajo ruido: navegación lateral, editor central, paneles acoplables para agentes/procesos/recursos y paleta de comandos. Tokens CHXCHX coherentes sin anular controles nativos de cada sistema operativo.
- Lecturas de proyecto, agentes e historial bajo demanda. Tareas externas y mediciones corren asíncronamente; el hilo visual nunca espera a procesos.
- CLI/TUI existentes mantienen su contrato mientras el frontend C++ consume servicios compartidos por un bridge local JSON versionado. No copiar reglas de trust, RAM Governor, selección de skills o ciclo de vida al código de UI.
- Medir startup, latencia de navegación, consumo idle y apertura de repositorios grandes. El núcleo C++ reemplaza la autoridad Python por dominios con pruebas de paridad; no se mantendrán dos implementaciones de producción.

### Progreso de implementación

- El shell Qt está en `native/`: navegación, árbol de archivos, pestañas y Scintilla/Lexilla con UTF-8, guardado atómico, números de línea, folding y lexers para los lenguajes principales del catálogo.
- `chxchx-tech bridge status PATH` publica `chxchx.project-status` versión 1. El snapshot de solo lectura cubre metadatos, trust/workspace, agentes, procesos, recursos del sistema, recursos administrados y umbrales del RAM Governor.
- Studio consume el contrato JSON de forma asíncrona para Inicio, Proyecto, Agentes, Procesos, Proyectos, Skills y Tech Packs. Recursos usa `bridge resources PATH` para tomar una muestra del sistema y resumir consumo de procesos gestionados por cada proyecto registrado. Skills muestra catálogo, selección activa y razones de recomendación; permite habilitar/deshabilitar y sincronizar. Tech Packs muestra composición y detección; permite aplicar un pack o todos los detectados y sincronizar. Todas las escrituras usan `--dry-run`, vista previa y confirmación.
- Agentes ya permite iniciar un agente, abrir chat nuevo con contexto o iniciar todos; Procesos permite iniciar/detener el elemento elegido. Las acciones pasan por `--dry-run` y confirmación de la UI. El CLI repite el preflight del RAM Governor al iniciar agentes y Studio pide confirmación con los avisos actuales antes de usar `--force`.
- Proyecto ofrece trust, start/resume, suspend y stop; Proyectos lista registros desde el bridge y cambia contexto sin adjuntar terminal. Los cambios usan `--dry-run`, confirmación visible y la lógica de recursos/trust del CLI.
- Handoff tiene formulario para resumen/pendiente/validación, lectura del contenido vigente y guardado por `workspace handoff` con preview/confirmación. Memoria busca notas locales por título/contenido y muestra su detalle en solo lectura; el bridge valida que las rutas permanezcan dentro del proyecto.
- Chats busca el historial local de Codex/Claude por título y contenido y carga una transcripción elegida bajo demanda. Errores muestra la caché local filtrada al proyecto y el detalle del registro; ambas vistas son de solo lectura.
- Configuración ya ejecuta preparación completa, init completo/mínimo, instalación de herramientas e integración MCP con `--dry-run`, vista previa y confirmación; Doctor queda como diagnóstico de solo lectura. Guía y Marca tienen vistas nativas. `Ctrl+P` abre una paleta filtrable para navegar áreas, archivos, buscar/guardar y disparar acciones disponibles de la vista. `Ctrl+F` busca dentro del archivo abierto.
- Studio ya ofrece terminales embebidas de agentes en pestañas con PTY/ConPTY y libvterm; la paleta conserva el emulador externo para Zellij/workspace/attach. El preflight de trust/RAM y el wrapper contextual compartido están implementados. La búsqueda actual cubre el archivo activo; siguen pendientes búsqueda global, historial de archivos, splits, diff Git, símbolos y LSP.
- Terminal embebida — transporte, emulación VT y pestaña de agente implementados en `native/src/integrations/pty_session.*`, `vt_terminal_widget.*` y `agent_session_widget.*`. Studio ya pinta celdas ANSI/VT, procesa teclado, resize y mouse/rueda. Trust/RAM se verifican con `agent preflight`; el bridge comparte el wrapper de instrumentación y contexto. Pendiente implementar scrollback navegable y pruebas de render/scrollback, compilar con dependencias disponibles y validar CI por plataforma. Conservar launcher externo como fallback.
- CI configura Qt 6.8.3 y compila y ejecuta el smoke de Scintilla en Ubuntu, Windows, macOS y Fedora. El build y `ctest` pasan localmente en Fedora con Qt 6.11.2; Windows/macOS requieren sus runners para validación real. La primera configuración CMake descarga revisiones fijadas de Scintilla/Lexilla y necesita acceso a GitHub; se documentan avisos en `native/THIRD_PARTY.md`.
- Siguen pendientes pruebas visuales y medidas de startup/RAM/latencia, empaquetado instalable por plataforma y migración del bridge transitorio a un núcleo C++ común. Completar antes el gobernador y la paridad de sesiones nuevas en la terminal embebida.

### Pendientes restantes

1. Completar terminal embebida: agregar scrollback acotado y navegable, pruebas de pantalla ANSI/VT y scrollback, y probar shell interactivo y `zellij attach`; verificar que resize y teclado sigan correctos. El transporte PTY/ConPTY, pintado libvterm y su smoke interactivo ya existen; mantener la acción de terminal externa como fallback.
2. Búsqueda global incremental que respete exclusiones (`.git`, dependencias, builds y `.ai` privado), con resultados que abran archivo y posicionen línea.
3. Lista de archivos recientes acotada al proyecto y pestañas restaurables de forma segura.
4. Splits con editor compartiendo documento sin duplicar estado sucio/guardado.
5. Git diff por archivo y navegación de símbolos; después añadir LSP mediante procesos del workspace.
6. Cerrar validación: smoke visual, startup/RAM/latencia, runners Windows/macOS/Fedora/Ubuntu y empaquetado.

### Núcleo compartido con CLI/TUI

- El estado objetivo es un núcleo de dominio C++ compartido por la app y las interfaces existentes, sin reglas de negocio duplicadas.
- La migración empieza con un bridge JSON local versionado a la implementación Python para conservar de inmediato todos los flujos actuales. El primer endpoint es `bridge status`; hay que versionar cualquier cambio incompatible y agregar operaciones de forma incremental. No es la arquitectura final ni una segunda fuente de verdad.
- Portar servicios por dominio y validar paridad antes de mover su autoridad: recursos/procesos, workspace/config/trust, agentes/adapters, skills/packs, memoria/historial/errors/handoff.
- CLI y TUI Python permanecen como frontends compatibles durante la migración; sus comandos delegan al núcleo compartido cuando cada servicio haya pasado la validación.
- Usar `QProcess` con lista de argumentos y ejecución asíncrona, nunca concatenar comandos del proyecto a través de un shell.

### Edición de código

- Scintilla proporciona edición C++ multiplataforma con selección, folding, márgenes, undo/redo e indicadores; Lexilla aporta lexers.
- Añadir lenguaje inteligente mediante Language Server Protocol después de estabilizar edición básica; el servidor de lenguaje vive fuera del proceso UI y se administra como proceso del workspace.
- Búsqueda de archivos/texto incremental, archivos recientes, pestañas, splits, diff Git y navegación de símbolos.

### Distribución

- Linux: validar Fedora y Ubuntu; ofrecer AppImage o Flatpak y paquetes nativos donde aporte valor.
- macOS: `.app`/DMG universal para Apple Silicon e Intel según la matriz Qt vigente.
- Windows: instalador firmado y runtime Qt empaquetado.
- No asumir que compilar en una distribución produce binarios compatibles con todas: probar dependencias, arquitectura, firma y actualización en CI por plataforma.
- Mantener una lista de avisos de licencias de Qt, Scintilla y Lexilla junto con sus avisos originales.

## Entregas

1. Crear el target CMake Qt Widgets y ventana nativa vacía para macOS, Windows, Fedora y Ubuntu; validar instalación de toolchain y build reproducible.
2. Implementar shell KDE-inspired y editor Scintilla mínimo con abrir/editar/guardar, pestañas, árbol de proyecto y paleta de comandos.
3. Definir y probar el bridge JSON versionado para Inicio → Proyecto → Recursos, sin bloquear el hilo visual.
4. Implementar agentes/procesos/proyectos con las mismas confirmaciones trust, RAM Governor y acciones actuales.
5. Implementar skills/packs, handoff, memoria, chats, errores, guía, marca, configuración y doctor.
6. Añadir LSP y herramientas de edición avanzadas tras la experiencia base.
7. Medir startup/RAM/latencia y validar matriz TUI completa en Linux, Windows y macOS antes de fijar v1.

## Criterios de aceptación

- Cada área y operación listada está presente o tiene una explicación de seguridad/lectura equivalente; ninguna se pierde al pasar de TUI a editor.
- Acciones sensibles mantienen los mismos controles de trust, `--dry-run`, backups, cuotas y confirmación RAM.
- Lecturas extensas tienen búsqueda, estado vacío, errores legibles y navegación de regreso.
- Progreso de operaciones largas no congela el editor; cerrar el panel no cancela ni duplica procesos externos.
- La UI/editor es nativa C++, no incorpora Chromium ni requiere que Sublime/Kate estén instalados.
- La ventana no se bloquea durante consultas, builds o inicio de agentes; usa límites y cancelación donde sea seguro.
- Los paquetes para cada plataforma no instalan secretos y conservan avisos de licencias/dependencias.
- C++ es dueño de las reglas del núcleo al finalizar la migración; CLI/TUI/editor consultan el mismo motor y estado.
- Durante la transición hay un solo dueño por operación y pruebas comparan la salida Python y C++ antes del cambio.
- Validación automatizada de la matriz y prueba visual real en Windows, macOS, Fedora y Ubuntu antes de dar la integración por completa.
