# Roadmap

## Estado de la versión de uso diario

La rama `dev` ya contiene el control plane Python (CLI/TUI) y ChxChx Studio, la
aplicación nativa C++20/Qt 6. Studio integra el Skill Registry, Tech Packs,
workspace/trust, RAM Governor, procesos, agentes, terminal PTY/ConPTY, memoria,
handoff, navegación, edición, búsqueda de nombres/contenido, archivos recientes
y restauración de grupos divididos, además de ajustes configurables.
El inventario de capacidades está en `.ai/EDITOR-PARITY.md`.

## Cierre para empezar a usar Studio en proyectos reales

1. Recompilar y abrir un repositorio de trabajo con `./scripts/dev-studio.sh
   /ruta/al/proyecto`; el script configura `CHXCHX_TECH_CLI` hacia el `.venv`
   local si existe. Si se abre directamente el binario, asegurar que
   `chxchx-tech` esté en el `PATH` o definir esa variable.
2. Recorrer Guía: Init/preparación con preview, recomendaciones y sincronización
   de skills, trust del proyecto y nueva sesión de agente con contexto.
3. Probar en un repositorio real y confiable: abrir/guardar archivos, buscar
   rutas, usar undo/redo y atajos, abrir una shell, iniciar y cerrar una sesión
   de agente, revisar handoff, y verificar trust/RAM antes de ejecutar.
4. Corregir los fallos concretos que aparezcan y confirmar que cerrar Studio no
   deja procesos inesperados. Registrar la validación manual en el handoff.

Esta revisión funcional en Fedora es el único paso que bloquea afirmar que el
usuario ya puede empezar a usar esta copia a diario. Si el proyecto se va a
usar también en macOS o Windows, ejecutar además el build y smoke nativos en
cada plataforma destino. El CI tiene trabajos configurados para Linux, Fedora,
macOS y Windows, pero cada plataforma requiere evidencia de una ejecución
verde; la configuración del workflow no equivale a que haya pasado.

## Pendientes de Studio; no bloquean el uso local básico

- Attach integrado a Zellij disponible desde la paleta de comandos. Falta
  validarlo manualmente con una sesión real; se conserva el launcher externo.
- Smoke Qt offscreen cubre render ANSI, navegación al inicio/fin, resize con
  historial y sesión PTY larga con entrada; falta QA visual del reflow real.
- Búsqueda de contenido implementada en segundo plano: omite binarios y archivos
  mayores de 1 MiB, con máximo de 64 MiB leídos y 100 coincidencias por consulta.
- Archivos recientes y grupos divididos restaurables: persiste hasta 20 rutas por
  proyecto y reabre solo archivos limpios que existan dentro de la raíz canónica.
  Terminales/agentes no se reabren automáticamente.
- Diff Git de archivo implementado como lectura acotada desde `HEAD`; no incluye
  archivos sin seguimiento ni cambios aún no guardados. Navegación básica de declaraciones
  disponible en Python, C/C++, JS/TS, Java y C#. Evaluar LSP después de medir su costo.
- Baseline del índice: el checkout de 341 archivos indexa en 4 ms y busca una
  ruta inexistente en p50 0.392 ms/p95 0.404 ms; fixture sintética de 10.000
  archivos indexa en 62–65 ms, busca una ruta inexistente en p50 9.00 ms/p95
  9.55 ms y escanea contenido en 99–100 ms. Falta medir primer frame y RAM de
  Studio en repos reales grandes, y fijar límites antes de publicar.
- CPack y CI generan ZIP para Windows, DMG para macOS y TGZ para Ubuntu desde Qt
  SDK. Fedora empaqueta TGZ con las bibliotecas Qt del sistema y publica una
  pre-release rodante con assets descargables anónimamente; el instalador valida
  el SHA-256 del manifiesto. Falta ejecutar el workflow actualizado y probar
  instalación/arranque en una segunda máquina Fedora.
- Decidir si el núcleo Python seguirá siendo la autoridad mediante el bridge o
  si comienza una migración gradual a un núcleo C++ compartido. No bloquear el
  uso de Studio con una reescritura sin una necesidad medida.

## Evolución posterior

- Memoria de proyecto resiliente: el wrapper previo al chat nuevo recupera
  solicitudes explícitas de los transcripts Codex/Claude locales del proyecto,
  las agrega a `.ai/memory/PROJECT_MEMORY.md` sin duplicarlas y las inyecta al
  prompt. Falta confirmar manualmente el ciclo en Studio; depende de que el
  transcript local siga disponible y no restaura sesiones completas.
- Roles de agente y workflows una vez que el flujo diario tenga validación real.
- ChxChx Insights y aprendizaje desde el trabajo, con controles de privacidad y
  promoción explícita de observaciones a skills.
- Importación externa de skills solo con normalización, revisión de seguridad y
  conservación de licencia/procedencia.

## Regla de convivencia entre interfaces

CLI, TUI y Studio operan la misma configuración y servicios del proyecto. Abrir
o cerrar una interfaz no debe iniciar, duplicar ni detener procesos de otra;
foco, pestañas y paneles pertenecen a cada interfaz.
