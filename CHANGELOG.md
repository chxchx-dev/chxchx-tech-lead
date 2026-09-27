# Changelog

## Próximo release — Fase 6 multiproyecto

- alias automáticos y resolución de proyectos por alias o ruta;
- persistencia del último proyecto activado;
- estados `ACTIVE`, `SUSPENDED` y `STOPPED`;
- `projects current`, `projects switch`, `workspace suspend` y `workspace resume`;
- recuperación de sesiones Zellij al reanudar un workspace.
- estado y disponibilidad de agentes con `agent list/status`;
- presets de agentes por proyecto;
- handoff administrado mediante `workspace handoff`.

## Unreleased

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
