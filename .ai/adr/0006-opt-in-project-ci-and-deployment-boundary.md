# ADR-0006: CI opt-in por proyecto y límite de despliegue

- Estado: CI implementado en CLI/TUI/Studio; CD a servidor pendiente del formato de runtime
- Fecha: 2026-10-14
- Alcance: ChxChx como control plane local para proyectos del usuario

## Decisión

- ChxChx ofrece un workflow GitHub Actions opcional para Python, Node.js y
  CMake/C++/Qt. Detecta los stacks y muestra una vista previa antes de escribir.
- Las acciones externas se fijan por SHA. El workflow usa `contents: read` y
  no crea secrets ni despliega automáticamente.
- La integración GitHub se limita al adapter `integrations/github_cli.py` y a
  operaciones oficiales de `gh`: estado de sesión, ejecuciones recientes,
  dispatch manual y logs fallidos.
- El archivo `.github/workflows/chxchx-ci.yml` se escribe solo si está ausente o
  contiene la versión íntegra administrada por ChxChx. Los cambios se respaldan
  fuera del proyecto; archivos ajenos o modificados requieren revisión manual.
- El bridge JSON v1 expone un snapshot de CI de solo lectura. CLI, TUI y Studio
  llaman los mismos casos de uso; Studio y TUI piden confirmación antes de
  escribir, retirar o iniciar una ejecución remota.
- La generación de CD a servidor propio/VPS queda opt-in y separada. Primero se
  define el runtime/empaquetado de los proyectos objetivo. No se altera el
  despliegue ni el pipeline de este repositorio.

## Validación y pendiente

- Cubrir generación Python/Node/CMake/Qt, integridad/backup, permisos de GitHub
  CLI y contrato bridge con pruebas locales y CI.
- Tras definir el runtime de despliegue, diseñar secrets fuera del repo,
  preflight/dry-run, rollback y confirmación antes de escribir o desplegar.
