# Roadmap

## Ahora

- v0.6 / Skill Engine: registro local con comandos `list`, `search`, `info` y `recommend` integrado al detector existente.
- Catálogo curado de 17 skills con búsqueda, consulta, recomendación y detección de stacks; incluye guía UI/UX avanzada para proyectos web y móvil.
- Selección por proyecto con `skill enable/disable` y carga explícita con `skill sync`.
- Tech Packs curados, razones de detección y aplicación aditiva implementados.
- RAM Governor v0: aviso de presión y cuota de agentes con confirmación desde TUI; sin parada automática.
- Tokens de color compartidos por CLI y TUI para alinear estados, navegación y acentos con una estética limpia y sobria.
- `editor setup` genera un `.sublime-project` local con exclusiones, `--dry-run`, idempotencia y backup al actualizar archivos administrados.
- Próximo entregable: probar visualmente los tres modos y ajustar consistencia; después medir el tamaño del contexto sincronizado.

## Después

- Roles de agente y workflows.
- Medir RAM reservable por agente y mostrar tendencia en Recursos; luego comandos desde Sublime, workflows y ChxChx Insights.
- Importación externa solo después de normalización, revisión de seguridad y preservación de licencia/procedencia.

## Modos de trabajo: CLI, TUI y editor

Objetivo: ofrecer interfaces separadas que ejecuten las mismas operaciones de workspace y respeten la misma configuración, política de recursos y estado por proyecto.

1. **Contrato compartido:** mantener los casos de uso en servicios de `workspace/`, `skills/`, `agents/` y `memory/`. CLI, TUI y editor llaman esos servicios; ninguna interfaz se convierte en dueña de sesiones, procesos o configuración.
2. **CLI estable:** completar los flujos de automatización y comandos operativos. La CLI debe poder ejecutarse sin abrir la TUI o el editor y mantener sus confirmaciones de RAM/cuota.
3. **Aplicación nativa ChxChx:** crear escritorio/editor C++20 + Qt 6 Widgets + Scintilla/Lexilla, con paridad funcional completa con TUI y objetivo macOS, Windows, Fedora y Ubuntu. El inventario está en `.ai/EDITOR-PARITY.md`.
4. Construir primero el shell Qt y editor mínimo, luego un bridge JSON versionado temporal para conservar reglas entre CLI/TUI/app sin congelar la ventana.
5. Migrar el núcleo compartido a C++ por dominios; CLI y TUI siguen disponibles y delegan al mismo núcleo conforme cada dominio supere pruebas de paridad.
6. Implementar áreas TUI por verticales medibles y luego LSP/funciones avanzadas de edición. Medir startup, RAM y latencia durante cada vertical.
7. Añadir CI/build/paquetizado multiplataforma para Fedora, Ubuntu, macOS y Windows y verificar la matriz completa.

Regla de convivencia: abrir o cerrar una interfaz no debe detener ni duplicar procesos del proyecto. El estado de workspace es compartido; el estado de pantalla, foco y paneles pertenece a cada interfaz.
