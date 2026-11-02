# ADR-0008: Studio como editor principal de TUI y CLI

- Estado: Aceptada
- Fecha: 2026-10-23

## Contexto

ChxChx Studio ya es el editor nativo del producto, pero la TUI y la CLI todavía
dirigían al usuario a Sublime Text y generaban archivos `.sublime-project`.
Instalar un editor externo dificultaba el primer uso y mantenía dos recorridos.

## Decisión

- Los proyectos nuevos configuran `editor = "studio"`; `studio open` abre Studio
  explícitamente y `editor open` usa el editor configurado.
- La paleta de la TUI abre ChxChx Studio y ya no ofrece preparar un proyecto de
  Sublime. El adapter Sublime queda disponible para proyectos que lo configuren
  durante la migración gradual.
- El launcher de Studio resuelve instalaciones user-local en Windows, macOS y
  Linux y abre Studio separado del proceso TUI.
- Bash sigue siendo el instalador común para macOS/Linux. Los paquetes llevan
  los runtimes Qt desplegables; en Linux, el instalador añade los componentes
  gráficos propios de Ubuntu/Fedora con el gestor del sistema.
- Windows ofrece `Setup.exe`, que contiene el script PowerShell existente. Ese
  flujo valida los assets, instala el bridge CLI y admite actualizaciones al
  volver a ejecutarse. El ZIP nativo permanece para el actualizador integrado.

## Consecuencias

- Studio es la experiencia recomendada al editar; las herramientas externas se
  mantienen donde todavía las necesita una configuración existente.
- La TUI informa si Studio no está instalado; no descarga ni actualiza software
  desde una acción de apertura.
- Las dependencias base del sistema operativo y la firma/notarización continúan
  fuera del bundle portable y requieren validación en máquinas limpias.
- La publicación del release preview conserva la política de rama y el estado
  aplazado registrados en ADR 0005.
