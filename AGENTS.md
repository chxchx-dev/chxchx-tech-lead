# AGENTS.md — chichan-tech-lead

## Propósito

Este repositorio construye un bootstrapper cross-platform para preparar repositorios de desarrollo asistido por agentes.

## Cómo se guía a la IA

La orientación del trabajo sigue una cadena explícita y visible:

1. La solicitud del usuario define el objetivo y sus límites.
2. `AGENTS.md` define las reglas estables de este repositorio.
3. `CLAUDE.md` añade únicamente ajustes específicos de Claude y hereda estas reglas.
4. `docs/03-ARCHITECTURE.md`, seguridad y desarrollo explican decisiones técnicas antes de cambiar código.
5. En los proyectos preparados, `.ai/PROJECT.md`, `.ai/CURRENT_STATE.md`, `.ai/ROADMAP.md` y `.ai/HANDOFF.md` aportan contexto operativo.
6. Basic Memory conserva decisiones recuperables y Serena ayuda a navegar el código cuando están disponibles.

El agente debe preferir las instrucciones más específicas sin contradecir las reglas de seguridad, arquitectura o la solicitud explícita del usuario.

## Protocolo de trabajo del agente

Antes de modificar:

- leer las reglas relevantes y el estado del proyecto;
- localizar la implementación, las pruebas y la documentación afectadas;
- proponer un cambio pequeño y verificable;
- usar `--dry-run` antes de cambios de configuración cuando exista esa opción.

Durante el cambio:

- preservar el trabajo existente y los bloques administrados;
- encapsular integraciones externas en `integrations/`;
- evitar secretos, cambios globales innecesarios y reestructuraciones amplias;
- actualizar la documentación cuando cambie el comportamiento público.

Antes de cerrar:

- ejecutar las verificaciones disponibles;
- informar qué cambió, qué se comprobó y qué queda fuera de alcance;
- dejar `.ai/HANDOFF.md` actualizado cuando el trabajo quede incompleto en un proyecto preparado.

## Reglas obligatorias

- Mantén los comandos idempotentes.
- Antes de modificar configuración del usuario, ofrece o respeta `--dry-run` cuando aplique.
- No almacenes secretos ni tokens.
- Prefiere CLI oficiales sobre edición directa de configuraciones de terceros.
- Si una integración cambia, encapsula el cambio en `integrations/`.
- No conviertas el proyecto en un framework de agentes: sigue siendo un orquestador/instalador.
- Toda modificación destructiva debe tener backup o una salida clara de rollback.
- Soporta Windows, Linux y macOS cuando la funcionalidad dependa solo de Python/uv.

## Calidad

Antes de cerrar cambios:

```bash
python -m compileall src
pytest
```

## Arquitectura

Ver `docs/03-ARCHITECTURE.md`.
