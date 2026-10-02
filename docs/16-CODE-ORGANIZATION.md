# Organización del código

## Objetivo

Mantener módulos pequeños, con una responsabilidad clara y fáciles de explorar
por una persona nueva en el proyecto. El límite operativo para código Python es
de 300 líneas por archivo; los archivos que superen ese límite deben tener una
excepción documentada y un plan de extracción.

## Capas

```text
cli/            comandos Typer y traducción de errores a salida CLI
tui/            composición de la interfaz Textual y eventos de usuario
core/           detección, configuración, registro, confianza y persistencia base
workspace/      casos de uso del workspace y modelos de ejecución
adapters/       wrappers de herramientas externas (Git, terminal, agentes, editor)
integrations/   Basic Memory, MCP e instaladores externos
```

Reglas de dependencia:

- `cli` y `tui` pueden invocar `workspace` y `core`, pero no implementan lógica
  de procesos ni integración externa.
- `workspace` coordina casos de uso; los comandos externos viven en `adapters`.
- `integrations` contiene efectos sobre herramientas o configuraciones externas.
- `core` no depende de Textual, Typer ni de adapters concretos.
- Los modelos y tipos compartidos deben vivir junto a la capa que los posee; no
  se crea un módulo `utils` genérico para ocultar dependencias.

## Estructura objetivo de la TUI

```text
tui/
├── app.py              # entrada pública: run_tui y error opcional
├── application.py      # clase Textual principal y ciclo de vida
├── palette.py          # paleta de comandos
├── screens/            # una pantalla por área funcional
│   ├── overview.py
│   ├── projects.py
│   ├── agents.py
│   ├── processes.py
│   ├── resources.py
│   ├── handoff.py
│   └── history.py
└── components/         # tablas, barras y widgets reutilizables
```

La TUI debe recibir servicios o fachadas pequeñas, en vez de importar y
construir todos los managers dentro de cada callback de pantalla.

## Plan de extracción

1. Extraer la aplicación TUI, paleta, estilos y grupos de acciones/eventos
   conservando atajos, IDs y textos visibles. **Hecho.**
2. Seguir separando las vistas TUI cuando cambie cada área funcional.
3. Dividir los comandos CLI por dominio manteniendo los mismos entry points.
   **Hecho.**
4. Dividir `WorkspaceService` por casos de uso y dejar una fachada compatible.
   **Hecho:** agentes en `agent_operations.py` y procesos en
   `process_operations.py`; `service.py` conserva la fachada.
5. Añadir una verificación de tamaño para evitar regresiones. **Hecho:**
   `tests/test_code_size.py` verifica todos los módulos Python de producción.

## Estado de módulos

La auditoría actual no detecta módulos Python de producción con 300 líneas o
más. `workspace.models` conserva imports compatibles y reexporta los tipos de
`config_models.py`, `config_validation.py` y `models_status.py`. El runtime de
procesos del sistema operativo vive en `process_runtime.py`.

La aplicación TUI ya está distribuida en `application.py`, `actions.py`,
`setup_actions.py`, `terminal_actions.py`, `events.py`, `dashboard.py`,
`panels.py`, `project_console.py`, `palette.py`, `branding.py` y `style.py`.
El caso de uso compartido de bootstrap vive en `core/bootstrap.py`. Los modelos
`ManagedProcess`, `ProcessActionResult` y `ProcessStatus` ahora viven en
`workspace/process_models.py`.
Zellij session tabs and layout construction are isolated in
`workspace/layouts.py` and `workspace/zellij_tabs.py`.

`cli.py` conserva el punto de entrada público; `commands/` registra comandos
por dominio y `cli_context.py` mantiene las apps Typer y las operaciones
compartidas de presentación/resolución de proyecto.

Cada extracción debe ser pequeña, ejecutable de forma independiente y cerrar
con `compileall`, `pytest` y una prueba manual de `chxchx-tech tui .`.

## Convenciones para nuevos módulos

- Un módulo debe tener una responsabilidad principal y preferiblemente menos de
  200 líneas; 300 es el máximo operativo, no el objetivo.
- Una clase pública debe tener una razón clara para existir y una interfaz
  pequeña.
- Los efectos externos deben estar detrás de adapters o integrations.
- Los callbacks de UI solo coordinan: validan entrada, llaman un caso de uso y
  presentan el resultado.
- Los nombres deben describir el dominio (`process_table`, `workspace_actions`)
  y no la implementación (`helpers`, `misc`, `common`).
