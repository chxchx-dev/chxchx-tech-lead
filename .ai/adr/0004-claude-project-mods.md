# ADR 0004: Mods de Claude Code administrados por proyecto

- Estado: Aceptada
- Fecha: 2026-10-14

## Contexto

El usuario quiere flujos reutilizables para tareas comunes y un selector que
ayude a elegir y cambiar el modelo de Claude según el trabajo. Studio prepara el
proyecto desde una vista previa y no debe instalar código global de terceros.

## Decisión

- El catálogo ChxChx ofrece Mods como instrucciones Markdown compatibles con las
  skills locales de Claude Code, almacenadas en `.claude/skills/` del proyecto.
- El catálogo incluye asesor de modelo, revisión previa a integrar, depuración
  con evidencia, planificación de funcionalidades, guía C++/Qt, flujo Git,
  refactorización segura, documentación técnica y rendimiento basado en medidas.
- La instalación/remoción se limita a archivos marcados por ChxChx. `--dry-run`
  no escribe; los cambios reemplazados o retirados se guardan en backups dentro
  de `.ai/backups/claude-mods/`. Los archivos ajenos no se sobrescriben.
- `projects prepare` incorpora Mods recomendados solo cuando detecta contexto o
  un agente Claude, y Studio muestra el plan mediante su previsualización actual.
- El asesor recomienda un alias Haiku/Sonnet/Opus y entrega el comando oficial
  `/model <alias>`. La decisión de cambiar queda en manos del usuario dentro de
  Claude Code; el catálogo no promete un API no documentado para cambiar el
  modelo de una sesión en curso.

## Consecuencias

- Los Mods no ejecutan hooks, no instalan paquetes/plugins y no cambian settings
  globales. Claude Code los carga en una sesión nueva.
- La función se limita a Claude Code; Skills y Tech Packs siguen siendo el
  contexto común que puede sincronizarse para otros agentes.
- Se puede añadir soporte a otros clientes con adapters separados sin cambiar
  este formato ni mezclar políticas del proveedor.
