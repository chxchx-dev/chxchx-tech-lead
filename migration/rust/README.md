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

En la TUI, `i` muestra el plan de init completo, `m` el compacto, `s` muestra
sync, `y` aplica el plan abierto, `c` abre MCP, `d` ejecuta el diagnóstico y
`p` muestra proyectos registrados. En el preview de sync, `↑/↓` desplaza y `Esc`
cancela. `r` actualiza y `q` sale. Studio ofrece botones para init, sync,
rollback, trust, MCP, proyectos registrados y doctor. Sigue siendo un shell
nativo para evaluar la biblioteca; todavía no es el editor completo. Iced usa
renderer software.
La compilación local se comprobó en Windows; macOS, Ubuntu y Fedora siguen
pendientes en CI/QA. La compatibilidad de JSON es una versión nueva e independiente
del bridge completo `chxchx.project-status` v1 del producto legado.
