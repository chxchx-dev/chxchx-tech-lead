# chichan-tech-lead

Bootstrapper cross-platform para preparar repositorios de desarrollo asistido por agentes.

La release `v0.2.0` es una base estable para uso propio: detecta el stack de un proyecto, genera configuración reproducible, instala herramientas gestionadas, conecta clientes MCP y mantiene un registro de varios proyectos sin guardar secretos.

## Estado de la release

`v0.2.0` está lista para probarse primero en un proyecto laboratorio y después incorporarse gradualmente a proyectos reales. El alcance es deliberadamente operativo; no convierte este repositorio en un framework de agentes.

Incluye:

- perfiles `web`, `python`, `node`, `dotnet`, `java`, `go`, `rust`, `data` y `generic`;
- detección de stacks y bases de datos, incluido PostgreSQL;
- instalación idempotente de Basic Memory y Serena mediante `uv`;
- `--dry-run`, backups y rollback para cambios gestionados;
- integración MCP con Claude, Codex y OpenCode;
- registro y sincronización de múltiples proyectos;
- soporte para Linux, macOS y Windows cuando la funcionalidad depende de Python/`uv`.

## Cómo se guía la IA

La orientación no depende de un prompt oculto. Está versionada junto al proyecto:

- `AGENTS.md` contiene las reglas estables, el protocolo de trabajo y las verificaciones obligatorias.
- `CLAUDE.md` hereda esas reglas y añade solo comportamiento específico de Claude.
- `docs/03-ARCHITECTURE.md`, `docs/06-SECURITY.md` y `docs/04-DEVELOPMENT.md` explican límites, riesgos y calidad.
- Al preparar otro repositorio, `chichan init` genera `.ai/PROJECT.md`, `.ai/CURRENT_STATE.md`, `.ai/ROADMAP.md`, `.ai/HANDOFF.md` y bloques administrados para agentes.
- Basic Memory conserva decisiones y Serena aporta navegación semántica; son apoyos del flujo, no sustitutos de las reglas del repositorio.

El resultado es una cadena reproducible: contexto → reglas → decisión → cambio pequeño → validación → handoff.

## Instalación estable desde Git

Requisito: tener instalado [`uv`](https://docs.astral.sh/uv/).

Linux y macOS:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
```

Instala la versión estable etiquetada. Sustituye `TU_USUARIO` por el propietario real del repositorio:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chichan-tech-lead.git@v0.2.0"
chichan version
chichan doctor
```

La salida de `chichan version` debe ser `0.2.0`. Para instalar desde un clon local durante desarrollo:

```bash
uv tool install --editable .
```

## Primer uso en un proyecto laboratorio

Ejecuta estos comandos desde la raíz del proyecto que quieres preparar:

```bash
chichan doctor
chichan install
chichan init --dry-run
chichan init
chichan status
```

`chichan init` crea o actualiza `.ai/` y registra el proyecto en `~/.chichan-tech-lead/projects.json`. La primera ejecución real conserva backups de la configuración gestionada.

Para conectar un cliente MCP, inspecciona primero el plan y luego aplica el cambio:

```bash
chichan integrate --dry-run --client claude
chichan integrate --client claude
```

Usa `--client codex` u `--client opencode` según el cliente que tengas instalado. La integración no instala esos clientes ni almacena tokens.

## Comandos principales

```text
chichan version                    Muestra la versión instalada.
chichan doctor                     Revisa requisitos y clientes detectables.
chichan install                    Instala herramientas gestionadas.
chichan init [PATH]                Prepara un proyecto y registra su metadata.
chichan setup [PATH]               Ejecuta la preparación completa.
chichan status [PATH]              Muestra el estado del proyecto.
chichan rollback [PATH]            Restaura el último backup gestionado.
chichan integrate [PATH]           Configura MCP para un cliente.
chichan projects list              Lista proyectos registrados.
chichan projects sync              Revalida y actualiza sus metadatos.
```

`PATH` es opcional y por defecto usa el directorio actual. Para cualquier comando que pueda cambiar configuración de usuario, usa primero `--dry-run`.

## Configuración y seguridad

- `.ai/` vive dentro de cada proyecto preparado.
- `~/.chichan-tech-lead/projects.json` contiene el registro local de proyectos.
- `~/.chichan-tech-lead/backups/` contiene copias de seguridad de cambios gestionados.
- Los perfiles se incluyen dentro del paquete y pueden sobrescribirse con `--profile-file`.
- No se guardan claves, tokens ni credenciales.

## Desarrollo local

```bash
uv sync --dev
uv run pytest -q
uv run python -m compileall -q src tests scripts
uv run python scripts/lab_smoke.py
```

La guía detallada está en [docs/01-QUICKSTART.md](docs/01-QUICKSTART.md). El uso avanzado está en [docs/02-USAGE.md](docs/02-USAGE.md), la publicación en [docs/10-PUBLISH-GIT.md](docs/10-PUBLISH-GIT.md) y los cambios por versión en [CHANGELOG.md](CHANGELOG.md).

## Roadmap

El trabajo posterior a `v0.2.0` se mantiene en [docs/05-ROADMAP.md](docs/05-ROADMAP.md). La guía de rollout para esta release está en [docs/13-PRODUCTION-PLAN.md](docs/13-PRODUCTION-PLAN.md).
