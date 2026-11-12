# Plan de reestructuración y migración a Rust

- Fecha: 2026-11-12.
- Estado: migración incremental en `migracion-rust`; ya hay slices compartidos
  de status/detección, preparación, rollback local y trust. La paridad total y el
  cambio de autoridad siguen pendientes.
- Dirección solicitada: migrar núcleo, CLI, TUI y Studio a Rust.
- Restricción confirmada: Studio nativo, sin WebView.
- Organización confirmada: un solo producto organizado por capacidades; CLI,
  TUI y Studio no serán proyectos ni crates independientes.
- Decisión asociada: [ADR 0016](adr/0016-rust-control-plane.md).
- Ejecución: [ROADMAP.md](ROADMAP.md). Versiones y evolución:
  [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md).
- Inventario inicial del port: [RUST-CAPABILITY-MATRIX.md](RUST-CAPABILITY-MATRIX.md).
- Primer schema versionado: [project-overview-v1.schema.json](../schemas/project-overview-v1.schema.json).

## 1. Resultado buscado

Un producto con un único motor de operaciones y tres experiencias coherentes:

La unidad de organización y entrega es la capacidad del producto, compartida
por los modos de interacción. CLI, TUI y Studio son presentaciones de una misma
aplicación, con el mismo estado, configuración, versión y ciclo de actualización.

| Interfaz | Responsabilidad |
| --- | --- |
| CLI | Automatización, diagnósticos, scripts y salida estructurada estable. |
| TUI | Operación del workspace desde terminal, con navegación por teclado. |
| Studio | Trabajo visual: archivos, edición, Git, terminal y chat de agentes. |

El producto conserva su responsabilidad de control plane local para herramientas
existentes. La reestructuración cambia la implementación y la organización;
no agrega un framework de agentes ni un servicio remoto obligatorio.

El destino elimina Python y Qt/C++ como dependencias del propio producto, una
vez completada la paridad. Las herramientas externas mantienen sus runtimes.
Rust no garantiza por sí solo menor RAM, mejor UX o builds sencillos: esos
resultados necesitan mediciones y validación por plataforma.

## 2. Punto de partida comprobado en el checkout

- CLI Python/Typer y TUI Textual; `pyproject.toml` declara versión `0.5.4`.
  El estado operativo documenta un release anterior `0.5.3`; no son la misma
  referencia y no se usará el historial como evidencia de pruebas actuales.
- Casos de uso repartidos entre `core/`, `workspace/`, `adapters/` e
  `integrations/`. `WorkspaceService` ya separa varias operaciones de la UI.
- Studio en `native/`, C++20/Qt 6, Scintilla/Lexilla y PTY/ConPTY + libvterm.
- Contratos JSON v1 en `workspace/bridge_contracts.py`; chat normalizado en
  `integrations/agent_stream.py` y drivers en vivo por proveedor.
- Git tiene comportamiento propio en `native/src/git/`, autorizado por ADR 0011.
  Por tanto, migrar solo Python dejaría funciones del producto sin portar.
- Catálogos distribuidos como recursos Markdown/TOML dentro del paquete Python.
- Instaladores, manifiesto, actualización y rollback definidos en ADR 0013;
  hoy `pyproject.toml` es la fuente de versión que también consume CMake.
- Inventario de capacidades en [EDITOR-PARITY.md](EDITOR-PARITY.md), con
  información histórica que deberá contrastarse con código, tests y QA real.

## 3. Reglas de diseño

1. Una autoridad por operación. Las interfaces presentan datos y solicitudes;
   no deciden trust, presupuestos, comandos de agentes ni reglas de backups.
2. Dominio separado de filesystem, procesos, IPC y bibliotecas visuales.
3. Integraciones externas encapsuladas bajo módulos `integrations/`.
4. Operaciones sensibles: planificar → mostrar efectos → confirmar → aplicar.
   `--dry-run` no escribe configuración ni inicia procesos configurados.
5. Idempotencia, escrituras atómicas y conservación de bloques administrados.
6. Trust local antes de ejecutar procesos configurados. Revalidar configuración
   al aplicar una operación para impedir usar una aprobación de una versión vieja.
7. Concurrencia explícita: abrir/cerrar una interfaz no controla implícitamente
   los procesos de otra. Cerrar un chat sí aplica su política explícita de cancelación.
8. Trabajo asíncrono, límites de salida/memoria y cancelación; ningún hilo de UI
   espera procesos o escaneos largos.
9. No guardar secretos, tokens, `.env` ni transcripciones en logs o checkpoints.
10. No exigir que las tres interfaces tengan la misma disposición; sí las mismas
    capacidades, controles de seguridad y resultados de los casos de uso comunes.
11. Organizar el código y el backlog por capacidades. Cada cambio incluye sus
    reglas, integraciones, pruebas y presentaciones aplicables; no abre tres
    implementaciones ni tres roadmaps para CLI, TUI y Studio.

## 4. Organización propuesta

Un repositorio, un paquete Cargo de producto y una biblioteca interna compartida
como punto de partida. Un workspace puede contener ese único paquete y centralizar
versión/perfiles, sin crear `chx-cli`, `chx-tui` ni `chx-studio`. Extraer un crate
en el futuro requiere una frontera técnica medida y un ADR; no se hará por tener
otra interfaz. El árbol siguiente es el destino, no una orden de mover hoy el legado.

```text
Cargo.toml                     # paquete único; versión común tras el corte
Cargo.lock
src/
  lib.rs                      # API interna común del producto
  main.rs                     # chxchx-tech: CLI, tui y studio
  bin/chxchx-studio.rs         # launcher fino compatible, si se necesita
  app/                        # arranque, composición y catálogo de acciones
  features/                   # capacidades compartidas, no pantallas
    projects/                 # configuración, preparación y trust
    workspace/                # procesos, recursos y sesiones
    agents/                   # chat, permisos, modelos y adjuntos
    knowledge/                # skills, packs, mods, memoria y contexto
    source_control/           # operaciones Git y planes seguros
    automation/               # CI/CD opt-in de proyectos
    distribution/             # canales, versión, actualización y rollback
  runtime/                    # dueño local de procesos y coordinación
  protocol/                   # DTO y contratos JSON/eventos versionados
  integrations/               # filesystem, procesos y herramientas externas
  interfaces/                 # solo parsing, composición visual e interacción
    cli/
    tui/
    studio/
assets/                       # catálogos, plantillas, iconos y temas
schemas/                      # contratos públicos versionados
tests/                        # escenarios por capacidad y fixtures compartidas
scripts/                      # desarrollo, paquetes e instaladores
.ai/                          # arquitectura, ADR y estado versionados
```

Dentro de cada capacidad separar `model`, `policy`, `service` y `ports` cuando
sean necesarios. No crear archivos vacíos para cumplir una plantilla. El dominio
es puro; el caso de uso usa puertos; `integrations/` los implementa; `app/`
compone. `runtime/` coordina procesos; `protocol/` no expone tipos gráficos.
Las capacidades nunca importan `interfaces/`. Los módulos de presentación
consumen una fachada común y modelos de lectura compartidos, sin lanzar procesos
o decidir permisos desde callbacks. La CI revisará estas fronteras además de
compilar: un solo paquete no las impone automáticamente.

Ejemplo: `agents` define iniciar/cancelar/autorizar y sus eventos una sola vez.
CLI convierte argumentos en solicitudes; TUI y Studio presentan esas mismas
solicitudes, planes y estados con sus componentes específicos. Layout, foco,
cursores y widgets pertenecen a presentación; los tres modos no necesitan
compartir widgets ni dependencias gráficas para compartir comportamiento.

Un catálogo común de acciones declara identificador, capacidad, requisitos y
soporte de preview; la CLI y las paletas usan ese catálogo para evitar flujos
divergentes. La paridad exige acceso a las operaciones comunes; no obliga a que
la CLI imite un editor visual o a que TUI pinte ventanas.

Dependencias gráficas opcionales por features Cargo: `cli`, `tui`, `studio`.
Un build CLI sin Studio no debe requerir GPU, Qt ni bibliotecas de ventana;
la edición completa habilita los tres modos de la misma versión. Mantener los
comandos públicos `chxchx-tech`, `chxchx-tech tui` y `chxchx-tech studio open`;
el launcher compatible `chxchx-studio` usa la misma biblioteca. Si Windows
necesita un ejecutable GUI sin consola, sigue siendo un launcher fino del mismo
paquete. Cambiar de modo no fusiona procesos ni transfiere permisos implícitamente.

Durante la transición Rust se construye bajo `migration/rust/` para no ocupar
el `src/` Python actual. `native/` permanece operativo. Promover el árbol Rust a
raíz y retirar esos árboles es una tarea del corte final con rollback probado.
No queda una carpeta permanente por implementación antigua.

## 5. Stack candidato y decisiones pendientes

| Área | Propuesta | Condición antes de adoptarla |
| --- | --- | --- |
| CLI | `clap` | Conservar comandos, opciones, códigos de salida y JSON público. |
| TUI | `ratatui` + `crossterm` | Teclado, resize, restauración del terminal y Windows real. |
| Studio | `iced`, candidato principal | Prototipo aprobado de edición, terminal, accesibilidad y paquetes. |
| Alternativa visual | `egui`/`eframe` | Comparar con el mismo prototipo si Iced no cumple. |
| Asincronía | `tokio` | Separar tareas bloqueantes y limitar colas/eventos. |
| Serialización | `serde`, `serde_json`, TOML | Preservar configuración existente y campos ajenos al producto. |
| Terminal | `portable-pty` + emulador VT evaluado aparte | Probar ConPTY, POSIX PTY, señales, resize y cierre. |
| Emulación VT | Evaluar `alacritty_terminal` | Render propio integrado con la GUI, compatibilidad y licencia. |
| Git y herramientas | CLI oficiales con argv estructurado | Paridad con backups, serialización y credenciales actuales. |
| Persistencia inicial | TOML/JSON y Markdown existentes | Sin convertir obligatoriamente los proyectos a otra base de datos. |

Iced se describe oficialmente como software experimental. Su `text_editor`
ofrece entrada multilínea; eso no demuestra paridad con Scintilla. La selección
visual sigue abierta hasta medir el costo de folding, undo/redo, resaltado,
archivos grandes y navegación. No se construirá un motor de editor desde cero
como supuesto implícito: si faltan capacidades, se documenta costo/alternativa
antes de comprometer la migración completa del Studio.

Una GUI Rust sin WebView puede dibujar controles propios; no implica widgets
nativos de cada sistema ni accesibilidad automática. IME, lectores de pantalla,
escalado, portapapeles y atajos forman parte de la evaluación.
`portable-pty` transporta entrada/salida; no pinta ni emula una terminal.

El prototipo inicial usa Clap, Ratatui, Iced y un renderer nativo software Iced.
Se fijaron versiones en `migration/rust/Cargo.toml`/`Cargo.lock` para reproducir
el build. Esto valida solo la compilación de arranque en Windows; aún no cierra
la evaluación visual, de editor, accesibilidad o soporte por plataforma.

## 6. Runtime y convivencia de interfaces

Propuesta a validar en la fase de procesos: un supervisor local por proyecto,
iniciado bajo demanda para procesos persistentes y compartido por las interfaces.
Las lecturas/transformaciones sin estado pueden ejecutarse en proceso mediante
los mismos casos de uso. No se instala un daemon global al arrancar el sistema.

- Identidad de proyecto basada en raíz canónica; exclusión mutua al crear dueño.
- IPC local: socket Unix o named pipe Windows, restringido al usuario, sin puerto
  TCP público. Handshake de versión/capacidades y rechazo de versiones incompatibles.
- El supervisor posee procesos administrados y sus estados; una conexión UI no
  equivale a una sesión de agente. Desconectar no reinicia ni duplica procesos.
- Locks de escritura por proyecto y por repositorio Git; no lanzar dos escrituras
  Git simultáneas. Detectar estado obsoleto antes de aplicar un plan aprobado.
- Recuperación tras crash con identidad verificable del proceso, no solo PID;
  nunca adoptar o matar un PID reutilizado. Probar caída del dueño y reconexión.
- Salida acotada, contrapresión y snapshots para clientes lentos; eventos de
  permisos no se pierden silenciosamente. Denegar por defecto si no hay respuesta.
- Definir parada explícita, política idle y qué sesiones sobreviven a cada cierre.
  No retirar el runtime anterior hasta aprobar esta matriz en los tres sistemas.

Esta separación no cambia los permisos del agente: cada adapter conserva el
sandbox y el protocolo de aprobación real que soporte el proveedor.

## 7. Estrategia de transición y datos

- Mantener el árbol Python/Qt operativo durante la migración. Cada operación tiene
  una tabla de autoridad: legado o Rust, nunca ambas escrituras en producción.
- Conservar el bridge JSON v1 y `chxchx.agent-event` v1 cuando el comportamiento
  sea compatible; nuevos contratos incompatibles reciben otra versión.
- El CLI legado puede delegar operaciones migradas al ejecutable Rust mediante
  un adapter en `integrations/`. No crear un ciclo Rust → Python → Rust.
- En QA, comparar resultados sobre fixtures o copias independientes. No ejecutar
  dos veces una mutación real para comparar implementaciones.
- Conservar `.ai/chxchx-tech.toml`, registros, trust local, Markdown, catálogos y
  bloques administrados. Inventariar también preferencias Qt/QSettings y rutas
  user-local antes de decidir qué importar al nuevo Studio.
- No importar trust de forma más permisiva: si la semántica/hash cambia, invalidar
  la confianza anterior y pedir una nueva aprobación explícita.
- Migración de formato solo con versión, preview, backup y rollback comprobado.
  Desconocer una versión debe producir un error accionable, nunca sobrescribirla.
- Inicialmente separar binarios/canales de prueba para evitar colisiones de PATH.
  El nombre público `chxchx-tech` cambia de dueño solo en un corte validado.
- Al cortar distribución, mover la fuente única de versión a Cargo y adaptar
  manifiestos/instaladores juntos. Hasta entonces sigue mandando `pyproject.toml`.

## 8. Fases y puertas de salida

| Fase | Entrega revisable | Criterio para avanzar |
| --- | --- | --- |
| 0. Inventario | Matriz capacidad → código/test → contrato → dueño → plataforma; baseline de recursos | Identificados comandos, datos, preferencias y flujos críticos; resultados actuales reproducidos. |
| 1. Viabilidad visual | Prototipo Rust descartable con árbol, editor, terminal, chat simulado y permisos | Editor/VT/IME/teclado/accesibilidad evaluados; build/arranque en Windows, macOS, Ubuntu y Fedora; ADR de GUI y costo pendiente. |
| 2. Base unificada Rust | Paquete único, fachada/catálogo de acciones, configuración y primera lectura en los tres modos | Un mismo `status` llega a CLI/TUI/Studio; fixtures coinciden, sin efectos ni reglas repetidas en UI. |
| 3. Mutaciones seguras | Init, preparación, trust, backups, skills/packs/mods con presentaciones aplicables | Dry-run sin efectos, segunda aplicación idempotente, rechazo de plan obsoleto, rollback probado en los tres recorridos. |
| 4. Runtime | Procesos, recursos, terminal y convivencia | Dos interfaces comparten dueño; límites, cancelación, crash, reconexión y cierre sin huérfanos en cada SO. |
| 5. Agentes y contexto | Adapters Codex/Claude/OpenCode/Ollama, memoria, historial y handoff | Contratos y falsos E2E pasan; aprobar/denegar/preguntar/cancelar y adjuntos validados; smoke real controlado. |
| 6. Flujo operativo completo | Git, modelos locales, CI/CD, diagnósticos y navegación sobre capacidades comunes | Paridad operativa; teclado/resize/errores/cancelación y terminal restaurado; un dueño de cada acción. |
| 7. Edición y experiencia integrada | Editor definitivo, paneles, paletas, Git/chat/terminal y preferencias | Matriz de paridad cerrada; QA visual y de consola en las cuatro plataformas, sin dividir el producto por interfaz. |
| 8. Distribución y retiro | CLI/TUI/Studio empaquetados; corte de autoridad y eliminación gradual del legado | Instalar → actualizar → rollback → desinstalar en equipos limpios; sin Python/Qt requeridos por el producto. |

Las fases 0–1 resuelven el riesgo antes de una reescritura extensa. Desde la fase
2, cada capacidad se entrega con sus presentaciones aplicables y pruebas comunes;
no se espera al final para empezar una reescritura independiente de TUI o Studio.
El prototipo visual no se publica como reemplazo.
No se fijan fechas sin inventario, capacidad del equipo y resultados del prototipo;
cada fase se divide después en PRs pequeños con evidencia de salida.

Git debe portar lo que hoy existe (ADR 0011/0012): cambios, historial, ramas,
stash, remotos, asistente y backups. LSP, stage por hunk y otras ampliaciones
siguen un backlog separado: no son requisitos nuevos escondidos en la migración.
CI/CD opt-in y modelos locales sí forman parte de la paridad existente.

## 9. Validación y rendimiento

- Mantener `python -m compileall src` y `pytest` mientras exista el legado.
- Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  y `cargo test --workspace`; añadir verificaciones de paquetes cuando existan.
- Dominio: políticas puras. Integraciones: repos temporales, argv y procesos
  falsos reales, siguiendo los E2E actuales; no depender de credenciales privadas.
- Contratos: schemas y fixtures del bridge/eventos; normalizar solo campos
  realmente variables (tiempo/PID/ruta), sin ocultar diferencias semánticas.
- Seguridad: trust ausente o cambiado, rutas fuera de raíz, symlinks/reparse
  points, cancelación con permisos pendientes, redacción y rollback fallido.
- Terminal/editor: Unicode, CRLF, entrada IME, colores, resize, scrollback,
  pegado, archivos grandes, guardado atómico y recuperación de error de disco.
- GUI/TUI: pruebas de estados y eventos más QA interactivo; una captura o smoke
  offscreen no acredita accesibilidad, fluidez ni comportamiento de ventana real.
- CI: Windows, macOS Intel/ARM según soporte comprometido, Ubuntu y Fedora.
  Fijar mínimos de SO/ABI; no asumir binario universal entre distribuciones.
- Antes del prototipo, medir arranque frío/caliente, RSS idle/activo, CPU idle,
  latencia p50/p95 de búsqueda/acciones y tamaño del paquete en el mismo equipo
  y repositorios. Fijar presupuestos con esa evidencia; bloquear regresiones
  relevantes y documentar cualquier excepción antes del corte.

## 10. Primer lote de trabajo ejecutable

1. Completar la matriz de fase 0 desde CLI, tests, Studio y ADR, corrigiendo
   únicamente discrepancias documentales comprobadas.
2. Crear el baseline reproducible del producto actual en el equipo disponible;
   separar evidencia local de resultados de CI y QA pendientes.
3. Especificar escenarios del prototipo visual y probar Iced con editor y VT;
   comparar Egui solo si aparecen límites que afecten los requisitos.
4. Completar el prototipo de árbol/editor/terminal/permisos, cerrar ADR de GUI,
   editor y propietario de procesos; después ampliar las capacidades de R2.

Alcance de esta entrega: planeación. La implementación, instalación del
toolchain Rust, publicación y retiro del producto actual son trabajo posterior.

## Fuentes técnicas de la evaluación

Consultadas el 2026-11-12; respaldan capacidades declaradas por los proyectos,
no sustituyen pruebas de integración ni determinan por sí solas la selección.

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html).
- [Clap](https://docs.rs/clap/latest/clap/).
- [Ratatui y backends](https://ratatui.rs/concepts/backends/).
- [Iced: características y estado experimental](https://github.com/iced-rs/iced).
- [Iced text_editor](https://docs.rs/iced/latest/iced/widget/text_editor/index.html).
- [Egui: frontend nativo](https://github.com/emilk/egui).
- [Portable PTY](https://docs.rs/portable-pty/latest/portable_pty/).
- [Alacritty terminal](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/).
