# ADR 0003: Preparación rápida de proyecto desde Studio

- Estado: Aceptada
- Fecha: 2026-10-14

## Contexto

La preparación de un repositorio requería recorrer Init, Skills, Tech Packs,
trust y Agentes en pantallas separadas. Inicio debe ofrecer una ruta corta para
entrar a trabajar, manteniendo las confirmaciones del control plane existente.

## Decisión

- Inicio ofrece `Preparar y entrar`, implementado como el caso de uso compartido
  `projects prepare PATH` y ejecutado por Studio a través del CLI.
- La vista previa precede a Init, la selección aditiva de skills recomendadas y
  skills de todos los Tech Packs detectados, la sincronización del contexto y el
  trust local. Tras aplicar los cambios, Studio inicia un chat nuevo con el
  agente elegido (o el primero configurado).
- El flujo no instala herramientas ni configura integraciones MCP globales.
  Si no existe agente, prepara el proyecto y explica que debe configurarse uno.
- Mods y selección de modelo Claude quedan fuera de este flujo hasta contar con
  un catálogo y una interfaz de configuración definidos. Ollama mantiene la
  selección guiada que presenta su CLI al iniciar.

## Consecuencias

- La preparación conserva la autoridad del CLI y sus adapters, backups y
  política de trust; Studio solo orquesta la vista previa, confirmación y chat.
- La operación es aditiva para skills: no deshabilita las que el proyecto ya
  tenía seleccionadas.
- Las funciones futuras de Mods pueden añadirse al caso de uso sin replicar la
  lógica de preparación en la interfaz.
