# Current State

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Chat visual con Codex y Claude (ADR 0009)

- `chxchx-tech agent chat` ejecuta un turno estructurado (`codex exec --json` / `claude -p --output-format stream-json`) y emite JSONL `chxchx.agent-event` v1. Requiere trust, envía el prompt por stdin, se cancela por stdin y escribe memoria y diario al terminar.
- Studio: botón **Chat visual** en Inicio, `Herramientas → Chat visual con agente` y paleta. Pasa por preflight de trust/RAM, abre una pestaña con mensajes en markdown, tarjetas de herramientas, permisos y Detener/Nuevo chat.
- `workspace/chat_context.py` concentra la preparación del contexto y la captura posterior, compartidas por la pestaña PTY y el chat visual.
- Validado con Codex 0.166 real en Fedora: dos turnos con reanudación, y el agente usó el resumen del diario. Claude no está instalado aquí; se implementó con su formato documentado.

## Contexto liviano y diario de sesiones (Fase 2)

- `workspace/context_compaction.py` mueve las secciones antiguas de `CURRENT_STATE`/`HANDOFF` a `.ai/history/` (se conservan 6 recientes, ~12.000 caracteres). Las secciones de `init` y los bloques administrados no se mueven. Hay backup vía `backup_project`. Se ejecuta antes de cada chat nuevo salvo con `auto_compact_context = false`, y a mano con `workspace context-compact`.
- `workspace/session_journal.py` resume al salir cada sesión de Codex/Claude en `.ai/history/SESSIONS.md` (local, ignorado por Git) y adjunta las 2 más recientes al prompt de chat nuevo. El prompt pide no leer `.ai/history/` salvo necesidad y no repetir historial.
- En este repo, "Estado actual" se condensó a mano; la versión extensa y los historiales están versionados en `.ai/history/` (`git add -f`).

## Memoria por chat confiable (Fase 1)

- La captura de memoria explícita solo acepta directivas al inicio de frase o tras "y"/"por favor" (ES/EN). Antes cualquier mención de "memoria" o "guarda" se guardaba e inyectaba en cada chat nuevo como petición literal.
- Se ignoran los bloques que el agente inyecta en el turno del usuario (`<system-reminder>`, selección del IDE, salida de comandos). Una misma petición repetida en otro chat se guarda una sola vez.
- La búsqueda de transcripts Claude usa el nombre de carpeta de Claude Code (todo carácter no alfanumérico → `-`, sufijo hash en rutas largas) y conserva el formato anterior. Como el nombre es ambiguo, cada transcript debe tener su `cwd` igual a la raíz del proyecto.
- `chxchx-tech workspace memory-prune PATH [--dry-run]` reevalúa las capturas automáticas de `PROJECT_MEMORY.md`, conserva las notas manuales y deja un backup fuera del repo.

## Instalación completa y TUI centrada en Studio

- Windows genera `chxchx-studio-windows-x86_64-setup.exe` con NSIS. El ejecutable incluye el instalador PowerShell, que descarga y valida el ZIP Qt y el bridge CLI; volver a abrirlo actualiza. El ZIP se mantiene para el actualizador integrado.
- Ubuntu y Fedora ahora requieren los mismos componentes Qt y el plugin xcb dentro del TGZ; el instalador común macOS/Linux añade con apt/dnf las bibliotecas gráficas del sistema. macOS conserva la app con frameworks desplegados en el DMG. El workflow comprueba los runtimes Qt para ambos paquetes Linux.
- La TUI abre ChxChx Studio en segundo plano y ya no ofrece preparar un proyecto Sublime. `chxchx-tech studio open` abre Studio; proyectos nuevos lo configuran como editor. Sublime sigue disponible si un proyecto lo configura explícitamente.
- Validación local: `python -m compileall -q src scripts`, `bash -n scripts/install-studio.sh scripts/install-studio-fedora.sh` y `git diff --check` pasan. No se ejecutó la suite. Este entorno no tiene PowerShell ni NSIS, así que el nuevo `Setup.exe` y el workflow todavía requieren build CI. No se probó instalación en equipos limpios; la publicación del preview sigue aplazada y el enlace de descarga aún no estará disponible hasta publicarla.

## README: guía de inicio para Studio y TUI

- El comienzo del README ahora ayuda a elegir Studio, la TUI o la CLI y ofrece pasos concretos para lanzar cada interfaz sobre un proyecto.
- La guía de Studio explica instalación/launcher, **Preparar y entrar**, edición, búsqueda, terminal, agentes y memoria. La TUI documenta preparación con `init`, trust, áreas principales, teclas y salida de Zellij.
- Se sustituyó el banner ASCII extenso y los bloques de uso de Studio/TUI se dividieron por recorrido y tema. Se mantienen las referencias técnicas y el catálogo de capacidades.

## OpenCode y memoria compartida entre agentes

- Studio ya permite añadir OpenCode u OpenCode con Ollama desde Agentes y conectar MCP de Basic Memory/Serena desde Configuración, con preview, confirmación y alcance local al proyecto.
- El wrapper ahora captura solicitudes explícitas desde sesiones OpenCode con `opencode session list` y `opencode export --sanitize`, valida que la sesión pertenezca a la ruta exacta del proyecto y comparte `.ai/memory/PROJECT_MEMORY.md` con Codex y Claude. Al iniciar chat nuevo, recupera y adjunta las solicitudes locales sin duplicarlas.
- No se lee la base de datos interna de OpenCode. La captura está acotada a las diez sesiones recientes y a cinco segundos por comando. La vista Chats permanece enfocada en Codex/Claude.
- No se ejecutaron pruebas en esta tarea; falta smoke manual con OpenCode normal y OpenCode con Ollama.

## Catálogo de Mods para Claude Code

- Se amplió de cinco a nueve guías locales con flujo Git, refactorización segura, documentación técnica y rendimiento basado en mediciones. Siguen siendo instrucciones instaladas por proyecto, sin hooks, ejecutables ni cambios globales.
- La recomendación de rendimiento depende de stacks/lenguajes detectados; las otras tres guías son de flujo general. README y ADR 0004 describen el catálogo actualizado.
- La instalación y actualización del preview de Studio se documenta en la sección siguiente.

## Studio: funcional; validación final para uso diario pendiente

- Studio tiene la funcionalidad principal implementada y los instaladores nativos pasaron CI: run #87 terminó con los 15 jobs aplicables en verde. La rama feature no publicó `studio-dev-preview`, de acuerdo con la decisión de aplazar esa publicación.
- Aún no se debe marcar como validado para uso diario. Falta recorrer en Fedora, con un proyecto confiable, Init y trust, skills/sync, edición y guardado, terminal, sesión de agente con contexto, preflight de RAM y cierre limpio. Tampoco hay evidencia de instalación interactiva en una segunda máquina Fedora.
- El menú **Ayuda → Buscar actualizaciones…** consulta el release y pide confirmación antes de aplicar. El actualizador valida los SHA-256, instala app y bridge CLI en una carpeta versionada, valida que el CLI nuevo arranque y cambia el launcher al final; conserva la versión anterior para rollback. El descubrimiento de instalación admite el marcador SHA completo y el marcador vacío de instaladores antiguos.
- Los instaladores Bash y PowerShell se pueden volver a ejecutar para actualizar; `--update`/`-Update` exige una instalación previa y `--dry-run`/`-DryRun` valida descargas y conflictos antes de escribir. Windows normaliza ZIP con raíz anidada. No se probó una instalación interactiva en equipos limpios; la publicación del preview continúa aplazada y el chequeo en Studio requiere que haya un release publicado.
- Verificación local: `python -m compileall -q src`, `bash -n` de instaladores, build nativo Qt y `git diff --check` pasan. No se ejecutó pytest ni una instalación; `pwsh` no está disponible localmente.

## Estado actual

- Producto: control plane local con CLI/TUI en Python y ChxChx Studio en C++20/Qt 6 Widgets con Scintilla/Lexilla. Studio llama al CLI mediante el bridge JSON v1 (`workspace/bridge_contracts.py`); Python sigue siendo la autoridad. La rama de integración es `dev`.
- Studio cubre todas las áreas de la TUI (inventario en `.ai/EDITOR-PARITY.md`). Toda escritura pasa por `--dry-run`, preview y confirmación. Inicio ofrece `Preparar y entrar`. Incluye terminal PTY/ConPTY + libvterm (scrollback de 5000 líneas), pestañas de agentes Codex/Claude/OpenCode (+Ollama), búsqueda de rutas y contenido, archivos recientes y grupos divididos, diff Git por archivo, símbolos básicos, atajos configurables y paleta `Ctrl+P`.
- Catálogo: 21 skills, 13 Tech Packs y 9 Mods de Claude Code en `.claude/skills`. RAM Governor: `max_agents` 2, aviso RAM 70 %, crítico 85 % y aviso swap 40 %; trust local obligatorio antes de ejecutar procesos.
- CI/CD de proyectos del usuario: CI opt-in para Python/Node/CMake y CD opt-in a VPS con Docker Compose (ADR 0007), ambos en CLI/TUI/Studio.
- Distribución: CPack genera ZIP/DMG/TGZ y un `Setup.exe` NSIS. Existe el release rodante `studio-dev-preview`. El run #87 de CI validó los paquetes en todos los runners. Nunca se ha probado la instalación en máquinas limpias.
- Memoria: `.ai/memory/PROJECT_MEMORY.md` guarda las peticiones explícitas, y Basic Memory se usa limitado al `memory_project` del proyecto. Al cerrar cada sesión de Codex/Claude se escribe un resumen en `.ai/history/SESSIONS.md`, que se inyecta en el siguiente chat nuevo. `CURRENT_STATE`/`HANDOFF` se compactan solos hacia `.ai/history/`.
- Lo que bloquea el uso diario: QA interactivo de Studio en Fedora con un proyecto real. El orden de trabajo está en `.ai/ROADMAP.md` (plan de noviembre 2026).
- El equipo actual (Fedora, noviembre 2026) no tiene cmake, g++ ni qt6-qtbase-devel, así que Studio no se compila aquí. El estado anterior completo está en `.ai/history/CURRENT_STATE.md`.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.
