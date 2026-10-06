# ADR-0002: Registro curado de skills con selección explícita

- Estado: Aceptada
- Fecha: 2026-10-04
- Alcance: biblioteca y contexto de skills por proyecto

## Contexto

ChxChx necesita añadir inteligencia operativa sin copiar catálogos externos completos ni cargar cada instrucción en todos los proyectos. La recomendación debe aprovechar la detección de stack existente y dejar el conjunto activo bajo control del usuario.

## Decisión

- La biblioteca inicial es local al paquete y cada skill contiene `skill.toml` y `SKILL.md`.
- Los Tech Packs son manifiestos TOML curados que agrupan IDs existentes de skills; cada coincidencia muestra los stacks o lenguajes que la activaron.
- El detector recomienda skills por stacks/lenguajes y siempre puede ofrecer unas pocas guías generales.
- `skill enable` y `skill disable` mantienen una selección explícita en `.ai/chxchx-skills.toml`.
- `pack apply` añade skills de un pack a la selección existente y es idempotente; no deshabilita selecciones manuales.
- `skill sync` carga solo lo seleccionado en `.ai/SKILLS.md` y añade a `AGENTS.md` una referencia en un bloque administrado.
- Las mutaciones admiten `--dry-run`, son idempotentes y crean un backup antes de reemplazar bloques o cambiar la selección existente.
- Las skills externas se importarán después con escaneo, normalización y datos de origen/licencia; importar no significará copiar sin revisar.

## Consecuencias

- El repositorio preparado recibe solo el contexto activado para su stack y sus necesidades.
- El CLI no ejecuta el contenido de las skills.
- Los perfiles de instalación, roles, workflows y aprendizaje continuo se mantienen como entregas separadas.
