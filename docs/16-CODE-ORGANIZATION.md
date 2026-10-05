# Organización del código

## Objetivo

Mantener ChxChx como instalador y orquestador local con tres interfaces: CLI,
TUI y Studio. Las interfaces deben traducir interacción a casos de uso; los
casos de uso coordinan dominio y adapters; los adapters encapsulan procesos y
herramientas externas. No se debe introducir un framework interno de agentes.

## Dirección de dependencias

```text
CLI / TUI / Studio
        ↓
casos de uso (core, workspace, skills)
        ↓
dominio y contratos
        ↓
adapters e integraciones externas
```

- `commands/` declara argumentos, invoca casos de uso y presenta resultados.
- `tui/` compone widgets, enruta eventos y llama los mismos casos de uso que CLI.
- `native/` contiene composición Qt; debe consumir contratos estables del CLI
  bridge y no duplicar reglas del workspace.
- `core/`, `workspace/` y `skills/` contienen reglas y casos de uso del producto.
- `adapters/` implementa protocolos de terminal, agentes, editor y Git.
- `integrations/` lee/escribe formatos de herramientas externas. Un formato por
  proveedor debe tener un lector propio detrás de una API estable.
- `ui/` mantiene estilos compartidos; no aloja lógica de negocio.

Evitar importaciones desde una interfaz hacia otra (por ejemplo, `core` desde
`tui` o `commands`). Las dependencias compartidas deben bajar a contratos o
casos de uso. Mantener la API pública existente mientras se mueven
implementaciones en pasos pequeños.

## Límites de módulos

- Mantener módulos Python por debajo de 300 líneas.
- Separar composición de UI, casos de uso y adapters; callbacks no contienen
  lógica de procesos ni edición de configuraciones externas.
- Crear módulos por responsabilidad o dominio. No añadir `utils.py`,
  `helpers.py` ni un nuevo punto central de importaciones globales.
- La composición puede conocer varias piezas; las reglas de dominio no deben
  conocer widgets, CLI ni detalles de clientes externos.
- Cada cambio de contrato compartido requiere cobertura en CLI, TUI y Studio.

## Estado y orden de extracción

### Paso completado

1. **Lectura de uso por proveedor:** `integrations/agent_usage.py` conserva la
   API pública y la selección de archivos. Los parsers de Codex y Claude están
   aislados en módulos propios; modelos y parsing común viven en módulos
   pequeños compartidos.

### Siguientes pasos

2. **CLI por dominio:** completada la eliminación de `cli_context.py`, que
   mezclaba imports, apps Typer, resolución de proyectos y presentación.
   `workspace/project_context.py` resuelve proyectos y construye
   `WorkspaceService`; `ui/cli_output.py` concentra presentación Rich;
   `cli_registry.py` define el árbol Typer; `cli.py` compone los comandos; y
   `commands/workspace_context.py` maneja inspección de workspace y creación
   del administrador de procesos para la CLI. Los grupos importan directamente
   dependencias de origen. La verificación de CLI ayuda a detectar imports
   inválidos al desacoplarlos.
3. **Composición de la TUI:** completada la separación de composición,
   enrutamiento y ciclo de vida. `tui/application.py` conserva la clase Textual,
   sus decoradores y bindings; `tui/event_dispatch.py` enruta eventos y
   `tui/lifecycle.py` configura tablas/títulos, temporizadores y refresco de
   pestañas. `tui/session_state.py` centraliza datos mutables en
   `WorkspaceSessionState`; descriptores conservan los atributos que consumen
   los mixins y escriben en una sola instancia. `layout.py` sigue dedicado a
   widgets y los mixins contienen handlers.
4. **Casos de uso del workspace:** conservar `WorkspaceService` como fachada
   pública. Cuando una operación crezca, mover un flujo completo a su módulo de
   caso de uso en `workspace/` antes de añadir otra clase base o mixin.
5. **Studio nativo:** el montaje de widgets salió de
   `native/src/main_window.cpp` a `native/src/main_window_layout.cpp`; las áreas
   de navegación se definen en `main_window_areas.hpp` y
   `integrations/bridge_client` encapsula ejecución y canales del proceso CLI.
   Las acciones están en `main_window_actions.cpp`, las vistas de handoff,
   memoria, chats y errores en `main_window_data_pages.cpp`, y la presentación
   de estado/recursos en `main_window_status_presenter.cpp`; selección y estado
   de acciones viven en `main_window_area_controller.cpp`. Python tipa los
   schemas superiores en `workspace/bridge_contracts.py`, Qt declara sus
   nombres en `integrations/bridge_schemas.hpp`, y su correspondencia queda en
   `docs/17-BRIDGE-CONTRACTS.md`. Mantener Qt en `native/` y las reglas de
   negocio en Python. Studio expone instalación/sincronización de skills,
   aplicación de packs y configuración MCP por cliente; todos los cambios
   siguen pasando por comandos CLI con preview y confirmación.

No realizar extracciones masivas. Antes de cada paso, identificar imports y
consumidores, mover una sola responsabilidad, conservar la interfaz, y revisar
las pruebas relacionadas y el límite de tamaño.

## Criterios de cierre de una extracción

- Los imports públicos existentes siguen funcionando.
- Cada módulo tiene una responsabilidad identificable y menos dependencias
  laterales.
- No se duplican reglas de dominio entre CLI, TUI y Studio.
- La documentación de este orden se actualiza al completar o reordenar un paso.
