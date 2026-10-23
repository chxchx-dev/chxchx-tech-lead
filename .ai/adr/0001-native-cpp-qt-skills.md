# ADR 0001: Catálogo para desarrollo nativo C++/Qt

- Estado: Aceptada
- Fecha: 2026-10-14

## Contexto

El producto ya incluye Studio nativo en C++20/Qt 6 y apunta a Windows, macOS y
Linux. El Skill Registry no identificaba C++ ni Qt, por lo que sus guías no
aparecían al recomendar instrucciones para esos repositorios.

## Decisión

El catálogo ofrecerá skills Markdown separadas para ingeniería C++, Qt, build y
empaquetado, y diferencias entre sistemas operativos. El detector marcará C++ a
partir de extensiones fuente conocidas y Qt a partir de marcadores en archivos
CMake, Meson o qmake. Las skills seguirán siendo opt-in mediante el flujo actual
de recomendaciones, packs y sincronización; detectar un stack no modificará la
configuración del proyecto.

## Consecuencias

- `skill recommend` y `pack detect` pueden sugerir guías nativas sin instalar ni
  ejecutar herramientas externas.
- Los marcadores son heurísticos: los nuevos formatos de archivo o sistemas de
  build requieren actualizar el detector.
- Cambios futuros en el catálogo deben actualizar documentación y conteos del
  registry.
