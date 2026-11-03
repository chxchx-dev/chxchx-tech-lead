# Changelog

## Unreleased

- El selector de agentes de Inicio ofrece **＋ Añadir OpenCode** cuando el proyecto no lo tiene (misma vista previa y confirmación que `agent add`). README con OpenCode y modelos locales, y capturas de ejemplo en `assets/screenshots/`.
- Pruebas en capas (`tests/README.md`): servidor HTTP falso de Ollama y ejecutables falsos de `ollama`/`opencode` para flujos E2E, y pruebas `live` opt-in (`pytest -m live`) contra las herramientas reales. Los agentes se lanzan por ruta completa para que los shims `.cmd` de Windows funcionen.
- Modelos locales: `local-models remove` (borra el modelo y sus variantes de Ollama y los quita de `opencode.json`), modelos cargados en memoria (`/api/ps`), botón Quitar en la pestaña de Studio y acceso desde Inicio.
- Studio añade la pestaña `Herramientas → Modelos locales (Ollama + OpenCode)…`: hardware, estado de Ollama, encaje de cada modelo, vista previa, descarga con confirmación y progreso, sobre `bridge local-models` y `local-models setup --json`.
- Añade `local-models status|setup` y `bridge local-models`: detecta RAM, GPU y disco, estima qué modelos de Ollama caben, los descarga con confirmación, crea una variante de 32K de contexto y los conecta al `opencode.json` del proyecto con `--dry-run` y backup propio. ChxChx no instala Ollama.
- Añade OpenCode como tercer agente del chat visual (`opencode run --format json`), con modos de solo lectura y edición, modelo, imágenes, uso y costo, y su sesión en el diario `.ai/history/SESSIONS.md`. Codex, Claude y OpenCode comparten `AGENTS.md`, `.ai/` y la memoria del proyecto, y cada uno funciona por separado.
- Identifica el build de pruebas de Terminal Workspace como `0.5.0.dev0` y documenta cómo actualizar instalaciones existentes desde `dev`.
- Escribe el estado persistente mediante archivo temporal y reemplazo atómico para tolerar interrupciones.
- Rota los logs de procesos al superar 5 MiB y conserva hasta tres copias anteriores.
- Añade pruebas de recuperación del estado de procesos tras reiniciar el manager.
- Actualiza roadmap y guía: la validación multiplataforma y la evidencia de uso real son los pendientes para v1.0.

- Renombra el proyecto, paquete y CLI a `chxchx-tech`, incluyendo su configuración, estado global, instaladores y documentación.
- Introduce el dominio inicial de Terminal Workspace y la configuración `chxchx-tech.toml` v2.
- Migra configuraciones v1 de forma idempotente y valida comandos, rutas relativas e IDs antes de cualquier ejecución.
- Añade confianza local y estado de workspace fuera del repositorio, con ejecución protegida de procesos.
- Añade el Process Manager y los comandos CLI iniciales `workspace status/trust` y `process list/start/stop`.
- Añade adapters aislados para Codex/Claude/OpenCode, Docker Compose y Git de solo lectura.
- Conecta el servicio de workspace con `open/start/stop/attach`, `editor open` y `agent start`.
- Añade `chxchx-tech resources` con RAM, swap, CPU, consumo de procesos y umbrales sin auto-kill.
- Añade dashboard TUI inicial con Textual, refresco de estado y acciones básicas del workspace.
- Fija el servidor MCP de Basic Memory al proyecto local para compartir conocimiento entre clientes.
- Añade `chxchx-tech agent start --all` y un header compacto configurable para sesiones Zellij.
- Añade banners ASCII por pane, reactivación de comandos suspendidos al adjuntar y orientación horizontal/vertical de agentes.
- Añade la receta `chxchx-tech run` para encadenar start, agentes y attach, más atajos raíz `attach` y `stop`.
- Actualiza los instaladores para preparar `uv`, ChxChx, Basic Memory y Serena de forma idempotente.
- Amplía `doctor`, README y la guía de uso con el flujo completo de instalación, trust y pruebas del workspace.
- Amplía la TUI con pestañas de resumen, proyectos, agentes, procesos, recursos y handoff, además de paleta `Ctrl+P` y acciones operativas.
- Completa las acciones del TUI para iniciar todo, controlar agentes/procesos, actualizar recursos y editar el handoff; añade el sello ASCII de CHXCHX-DEV.

## 0.2.0 - 2026-09-23

- Endurecimiento de `--dry-run`, backups, rollback e idempotencia.
- Perfiles TOML configurables y detección de PostgreSQL/AI.
- Comandos `projects list` y `projects sync`.
- Verificación posterior de integraciones MCP.
- Suite ampliada y smoke test de laboratorio.

## 0.1.0

- CLI inicial.
- Detectores básicos de stack.
- Estructura `.ai/` y documentación.
- Bloques administrados para AGENTS/CLAUDE.
- Instalación inicial de Basic Memory y Serena.
- Integración MCP inicial Claude/Codex.
- Diagnóstico, status, sync y rollback.
