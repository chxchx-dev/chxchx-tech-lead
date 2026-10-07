# Roadmap

## Estado de la versión de uso diario

La rama `dev` ya contiene el control plane Python (CLI/TUI) y ChxChx Studio, la
aplicación nativa C++20/Qt 6. Studio integra el Skill Registry, Tech Packs,
workspace/trust, RAM Governor, procesos, agentes, terminal PTY/ConPTY, memoria,
handoff, navegación, edición, búsqueda de nombres y ajustes configurables.
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

- Attach a una sesión Zellij desde la terminal integrada; por ahora se mantiene
  el launcher externo como alternativa.
- Añadir pruebas del render ANSI/VT, scrollback, resize y sesiones interactivas
  además de los smoke actuales de editor y PTY.
- Añadir búsqueda de contenido del proyecto; la búsqueda actual indexa nombres
  y rutas, no el texto de los archivos.
- Restaurar archivos recientes y grupos de pestañas de forma segura al iniciar.
- Añadir Git diff, navegación de símbolos y después integración LSP.
- Medir startup, RAM idle, latencia de navegación y apertura de repositorios
  grandes; definir límites aceptables antes de publicar.
- Crear paquetes instalables para Linux, macOS y Windows, con runtime y avisos
  de terceros incluidos.
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
