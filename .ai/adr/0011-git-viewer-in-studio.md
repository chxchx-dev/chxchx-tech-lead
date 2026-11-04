# ADR-0011: Visor de Git en Studio

- Estado: Aceptada (fase 1: cambios, historial y ramas; fase 2: enviar/recibir, stash y estado en el explorador)
- Fecha: 2026-11-02

## Contexto

Studio solo tenía un diálogo con el `git diff` de un archivo. Quien trabaja en un
proyecto necesita ver qué cambió, preparar y guardar cambios, recorrer el historial
y cambiar de rama sin salir del editor, con una interfaz más clara que la de VS Code.

## Decisión

- **Dónde**: una pestaña **Git** junto a **Archivos** en el dock izquierdo, con
  subpestañas Cambios, Historial y Ramas. El diff se abre como pestaña del editor
  (`GitDiffView`); `Herramientas → Diff Git del archivo actual` abre el mismo visor.
- **Motor nativo en C++** (`native/src/git/`): Studio ejecuta `git` con `QProcess`.
  Cada clic (preparar, cambiar de rama) debe responder al instante y el CLI de Python
  tarda ~0,3–0,4 s en arrancar. Esto se aparta de "Python es la autoridad", que ya se
  exceptuaba con el diff; Git es la autoridad de sí mismo y no hay reglas de producto
  (trust, recursos) que duplicar. Las acciones destructivas llevan confirmación y copia.
- **Capas**: `git_parsers` (texto → datos, sin procesos ni widgets), `git_repository`
  (cola serializada y asíncrona de comandos: los que escriben nunca corren a la vez
  y chocarían por `index.lock`; las lecturas usan `--no-optional-locks` para no bloquear
  el `git` que el usuario ejecute en su terminal), y las vistas
  (`GitChangesView`, `GitHistoryView`, `GitBranchesView`, `GitDiffView`, `GitPanel`).
  Los callbacks de las vistas que pueden cerrarse (pestañas de diff) usan `QPointer`.
- **Lenguaje**: español claro con el término Git entre paréntesis ("Listos para guardar
  (staged)", "Guardar (commit)"). Con nada preparado, el botón pasa a "Guardar
  todo (commit)", que prepara todo y guarda.
- **Seguridad**: descartar guarda antes una copia en `.git/chxchx-backups/<fecha>/`
  (`changes.patch` con todo el diff contra HEAD, y los archivos sin seguimiento) y se
  detiene si no puede guardarla; conserva las 20 últimas. Borrar una rama usa `-d` y solo
  ofrece `-D` tras una segunda confirmación. Cambiar de rama, crear y borrar delegan los
  errores en Git y los muestran sin recortar. Las rutas viajan por stdin
  (`--pathspec-from-file`), sin límite de línea de comandos ni inyección por nombres.
- **Detalles de Git**: estado con `status --porcelain=v2 -z --untracked-files=all`; un
  merge muestra lo que trajo respecto al primer padre (`-m --first-parent`); antes del
  primer commit, deshacer usa `rm --cached`; un archivo sin seguimiento se muestra
  como todo añadido.
- **Grafo**: los carriles se asignan en orden de visualización (`assignGraph`) y se
  pintan en un delegado; los merges son nodos huecos; hasta 8 carriles distintos.

## Fase 2

- **Enviar y recibir**: `fetch --all --prune`, `pull --ff-only` (por defecto), `pull --no-rebase --no-edit`
  (solo si el usuario acepta fusionar) y `push` / `push -u <remoto> <rama>` para publicar.
  Se aplican con 10 minutos de margen y se pueden cancelar. **No se fuerza nunca** un envío y
  **no se ofrece rebase**: ambos dejan al usuario en un estado difícil de deshacer desde la
  interfaz. Una fusión con conflictos puede abandonarse (`merge --abort`).
- **Credenciales**: Studio no las pide ni las guarda. Los procesos de Git arrancan en una sesión
  nueva (`CreateNewSession`), sin terminal de control, con `GIT_TERMINAL_PROMPT=0` y
  `GCM_INTERACTIVE=never`: `ssh` y los gestores no pueden quedarse esperando, fallan rápido y el
  mensaje explica qué configurar. Si el gestor del usuario muestra su propio diálogo gráfico, se respeta.
- **Problemas reconocibles** (`GitProblem`): credenciales, envío rechazado, ramas separadas, rama sin
  seguimiento, sin remoto, identidad y cancelado; la interfaz decide con ellos (ofrecer publicar o
  fusionar) y no con el texto de Git. Cancelar solo detiene operaciones lentas; si en ese momento
  corre una lectura rápida, se cancela la siguiente operación lenta.
- **Botón de sincronización**: una sola acción con el nombre de lo que hará según el estado
  (recibir, enviar, publicar, traer novedades). Enviar pide confirmación con "no volver a preguntar".
- **Stash**: sección dentro de Cambios (`stash push [--include-untracked] -m`, `apply`, `pop`,
  `drop`). Al eliminar se muestra el hash que permite recuperarlo con `git stash apply <hash>`.
- **Explorador**: `GitTreeDelegate` colorea los archivos cambiados, dibuja su letra y un punto en las
  carpetas que los contienen (`buildExplorerStatus`, sin dependencias de interfaz). Los archivos
  eliminados no aparecen porque ya no están en el árbol.

## Consecuencias

- Requiere Git 2.25 o posterior (`restore`, `switch`, `--pathspec-from-file`).
- Una carpeta dentro de un repositorio mayor usa la raíz del repositorio (`rev-parse`).
- No incluido (fase 3): preparar por bloques, blame, historial por archivo, resolver conflictos con
  un editor, búsqueda en el historial, `rebase` y envío forzado (decisión, no pendiente).
- Los remotos con autenticación interactiva solo funcionan si el usuario ya dejó configurado su gestor
  de credenciales o su llave SSH; Studio no lo configura.
- Probado con parsers sobre texto literal y con repositorios temporales reales
  (`git-smoke`, `git-ui-smoke`), incluidos dos clones contra un remoto local que divergen, chocan y se
  reconcilian. Sin probar contra un remoto real con autenticación, ni en Windows o macOS.
