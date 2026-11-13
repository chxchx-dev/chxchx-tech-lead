# ChxChx Rust product

Paquete único en desarrollo para la rama `migracion-rust`. La versión local es
`0.6.0-dev.0`; no se publica ni sustituye la instalación 0.5.x.

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
Su JSON es `chxchx.project-overview` v1 y está en `../../schemas/`.

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

Skills, Tech Packs y Mods están portados (CLI y Studio). El catálogo del legado
(21 skills, 13 packs, 23 mods) se empaqueta en el binario al compilar leyendo
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
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features cli --bin chxchx-tech -- status . --json
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features cli --bin chxchx-tech -- init .
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features cli --bin chxchx-tech -- init . --apply
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features cli,tui --bin chxchx-tech -- tui .
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features cli,studio --bin chxchx-tech -- studio open .
cargo run --manifest-path migration/rust/Cargo.toml --no-default-features --features studio --bin chxchx-studio -- .
```

## Verificación de CI

La suite de flujos compartidos se puede ejecutar sin compilar las interfaces
gráficas:

```powershell
cargo test --manifest-path migration/rust/Cargo.toml --locked --no-default-features --features cli
python -m pip install --requirement scripts/requirements-rust-schema.txt
python scripts/validate_project_overview_schema.py
```

Las pruebas aplican operaciones solo dentro de directorios temporales. El
validador ejecuta `status --json` con un home temporal y comprueba la salida real
contra `schemas/project-overview-v1.schema.json`.

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
cargo run --manifest-path migration/rust/Cargo.toml --bin chxchx-studio -- . README.md
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
