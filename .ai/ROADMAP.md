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
- Baseline preliminar sobre 341 archivos: indexación 4–5 ms, búsqueda de rutas
  p50 0.40 ms/p95 0.45 ms y búsqueda de contenido “TODO” mediana 15 ms (48 ms
  primer proceso frío). Falta medir primer frame, RAM/latencia en repos grandes
  y fijar límites antes de publicar; la muestra no representa repos grandes.
- CPack y CI ya generan ZIP para Windows, DMG para macOS y TGZ para Linux desde
  los runners con Qt SDK; cada paquete se publica como artefacto del workflow.
  Falta confirmar una ejecución verde por sistema y probar los artefactos en
  equipos destino. El TGZ construido con Qt de Fedora/Ubuntu puede depender del
  runtime Qt del sistema; no usar ese paquete local como distribución autónoma.
- Decidir si el núcleo Python seguirá siendo la autoridad mediante el bridge o
  si comienza una migración gradual a un núcleo C++ compartido. No bloquear el
  uso de Studio con una reescritura sin una necesidad medida.

## Evolución posterior

- Roles de agente y workflows una vez que el flujo diario tenga validación real.
- ChxChx Insights y aprendizaje desde el trabajo, con controles de privacidad y
  promoción explícita de observaciones a skills.
- Importación externa de skills solo con normalización, revisión de seguridad y
  conservación de licencia/procedencia.

## Regla de convivencia entre interfaces

CLI, TUI y Studio operan la misma configuración y servicios del proyecto. Abrir
o cerrar una interfaz no debe iniciar, duplicar ni detener procesos de otra;
foco, pestañas y paneles pertenecen a cada interfaz.
