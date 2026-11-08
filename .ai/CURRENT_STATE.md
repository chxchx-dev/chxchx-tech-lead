# Current State

## Release 0.5.3 preparado (2026-11-08)

- Versión estable fijada en `pyproject.toml` y `uv.lock`. Suite Python: 511 passed, 1 skipped, 10 deselected; compilación de Studio y `chxchx-agent-chat-smoke` pasan con Qt 6.11. El binario local informa `ChxChx Studio 0.5.3`; el tag coincide con la versión.
- Incluye el diagnóstico/registro local de errores de OpenCode y el pegado de imágenes con caché Markdown descritos abajo. Falta el CI/publicador remoto del tag y el QA visual con un modelo real.

## Pegado de imágenes en Studio (2026-11-08)

- El compositor acepta imágenes `image/*` aunque Qt no las marque con `hasImage()`, y archivos de imagen copiados como URL local. **✦** genera una descripción con el agente actual en `--lean`, solo lectura y sin sesión; la guarda atómicamente por SHA-256 en `.ai/cache/image-descriptions/` (30 días, máximo 200). Al reutilizarla, manda el Markdown en lugar de la imagen; conserva la opción de enviar el original cuando existe. Elimina la copia del portapapeles solo después de persistir la descripción; no toca imágenes fuente del usuario. Los turnos `--lean` limpian sus copias de `.ai/chat-attachments` al terminar.
- Smoke Qt cubre generación, caché, reutilización, envío de imagen original, borrado de la copia pegada, vencimiento y MIME crudo; OpenCode 1.x/2.x cubiertos con E2E. Suite completa: 511 passed, 1 skipped, 10 deselected; `compileall`, build de Studio y smoke Qt pasan. Falta probarlo desde una ventana real de Studio con un modelo de visión.

## OpenCode y registro de errores (2026-11-08)

- `agent add opencode` identifica la ruta del TOML faltante y muestra la inicialización con `--dry-run`. ChxChx registra errores de alta, preflight y chat en la caché local; `chxchx-tech workspace errors <PATH>` permite consultarlos, igual que `Más → Errores` en TUI/Studio. La caché redacta credenciales habituales.
- Para usar OpenCode en un proyecto sin configuración, inicialízalo primero y vuelve a añadir OpenCode; después vuelve a confiar el proyecto porque cambia el hash de configuración.
- Suite completa verificada: 511 passed, 1 skipped, 10 deselected. `python -m compileall src` y `git diff --check` también pasan.

> Historial anterior: `.ai/history/CURRENT_STATE.md` (no se lee al iniciar un chat).

## Chat visual: seguridad y acceso (ADR 0014)

- `agent_access.py` (niveles → banderas por CLI), `agent_permissions.py` (evento `permission`), `agent chat --network/--allow`. Studio: selector de 4 niveles, botón 🌐 Red, `ChatPermissionCard`, confirmación de acceso total, concesiones «una vez» / «en este chat» (`AgentChatWidget::answerPermission`).
- Fase 2 (Claude en vivo): `agent_live.py` (`LiveClaude`: control_request ↔ control_response), `agent chat --live`, `ChatQuestionCard`, `answerPermission`/`answerQuestion` escriben JSON por stdin. Probado con un `claude` falso que espera al host (`tests/e2e/test_claude_live_flow.py`, 3 mutaciones) y con Claude real.
- Codex en vivo: `agent_codex_live.py` (`LiveCodex`, cliente JSON-RPC de `codex app-server`), `codex_server_settings` (nivel → sandbox/aprobación), `answer` con `scope`. Probado con un `codex app-server` falso (`tests/e2e/test_codex_live_flow.py`, 4 mutaciones) y con Codex real (aprobar fuera del sandbox, denegar).
- OpenCode en vivo: `agent_opencode_live.py` (`LiveOpenCode`: `opencode serve` + SSE + replies HTTP, con contraseña por turno), `agent_chat._json_lines/_live_driver`. Probado con un `opencode serve` falso (`tests/e2e/test_opencode_live_flow.py`, 5 mutaciones) y con OpenCode real (permiso permitido y denegado, pregunta; sin procesos huérfanos).
- Pruebas: `tests/integrations/test_agent_access.py`, e2e de OpenCode, `agent_chat_smoke` (3 mutaciones detectadas). Verificado con claude/opencode/codex reales salvo `bypassPermissions` y `danger-full-access`.

## Instaladores y canales de release (ADR 0013)

- `scripts/install.sh` (Linux/macOS) e `scripts/install.ps1` (Windows): canal stable/dev, `--version`, `--update`, `--cli-only`, `--uninstall`, `--dry-run`, verificación SHA-256 con el manifiesto, nada ajeno se sobrescribe. `scripts/release_assets.py` genera manifiesto + `SHA256SUMS`. Wrappers: `install-studio.*`, `install-studio-fedora.sh`, `bootstrap.*`. `windows-studio-setup.nsi` empaqueta `install.ps1` (`-DCHANNEL`).
- CMake lee la versión de `pyproject.toml`; Studio responde `--version`/`--help` antes de Qt; iconos en `native/packaging/` (png/ico/icns, `.rc` en Windows). Updater Python (`studio_channels.py`, `studio_update_launcher.py`) sigue el canal instalado; corregido que `parse_manifest` rechazaba `schema_version`.
- CI: `stage-release` → `installer-smoke` (ubuntu, macOS x2, Windows, servidor localhost + `CHXCHX_TEST_BASE`) → `publish-studio-preview` / `publish-studio-stable` (tag `v*`, con `check-release-tag`). Firma condicionada a secretos (`WINDOWS_CERT_*`, `MACOS_*`, `APPLE_*`).
- Verificado en Linux: `tests/installers` (37) y `tests/integrations/test_studio_channels.py` (8), build local de Studio con `--version`. **No verificado**: `install.ps1`, NSIS, flujo DMG, `installer-smoke` y publicación real, firma (sin pwsh/macOS/certificados aquí).

## Instalación local stable (2026-11-04)

- ChxChx Studio `0.5.0` instalado desde el release oficial stable en Fedora x86_64 usando `curl -fsSL https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.sh | bash -s -- --no-deps`. El primer intento sin `--no-deps` se detuvo porque `sudo` requería una terminal; las bibliotecas de Qt ya estaban instaladas. `--dry-run` y la instalación verificaron el manifiesto y los SHA-256.
- Studio informa `ChxChx Studio 0.5.0`; el bridge informa `chxchx-tech-lead 0.5.0`. Launcher y entrada de menú presentes; `~/.local/bin` ya está en `PATH`. El instalador configuró `uv` y Python 3.11 para el entorno aislado.
- La instalación anterior `dev-5eae006`, su launcher y su entrada de menú se retiraron tras confirmar sus marcadores de ChxChx. No se abrió la ventana gráfica; el arranque visual y la instalación en equipos limpios siguen sin comprobarse.

## Asistente de Git: modelo ligero por defecto

- `git_assistant_models` (puro): `parseAgentOptions`, `looksLight` (por nombre o por tamaño ≤14b), `lowEffort`, `chooseModel` (automático / `__agent__` / un modelo concreto), `describeChoice`. El controlador lee la lista de modelos del agente (`bridge agent-options`) una vez y pasa `--model`/`--effort` a `agent chat --lean`; la tarjeta tiene el submenú Modelo (se reconstruye solo si algo cambió, para no cerrarse mientras se usa).
- Pruebas: elección pura en `git_smoke` (nombres, tamaños, esfuerzo, Haiku sin esfuerzo, modelo desconocido) y los argumentos reales de cada consulta en `git_ui_smoke`; 2 mutaciones detectadas; 12 ejecuciones del UI sin fallos.
- Sin probar con los modelos reales de cada CLI (¿Claude acepta `--effort low` con un modelo concreto?, ¿qué modelos ligeros publica Codex?).

## Asistente de Git y rediseño del panel

- Python: `agent chat --lean` (sin contexto/memoria/diario, solo lectura, sin sesión: `--ephemeral` Codex, `--no-session-persistence` Claude, borrado de la sesión en OpenCode) y `--prompt-file`; `ChatTurnPlan.lean`, `load_prompt`. 378 pruebas pasan.
- Nativo (`native/src/git/`): `git_assistant_prompts` (redacción de secretos y archivos sensibles, prompts, bloques de conflicto, limpieza del mensaje), `git_guide` (siguiente paso por reglas), `git_assistant` (ejecuta `agent chat --lean` con el prompt en un archivo temporal, cancelable), `git_assistant_controller` (agente, consentimiento, contexto, estado fresco de la fusión), `git_assistant_bar` (tarjeta con tono + selector de agente), `git_advice_dialog`. Botón Sugerir, "Pedir consejo" en la tarjeta y en los avisos de error.
- Diseño: iconos de Studio en pestañas, rama, sincronización, secciones, acciones de fila, diff y ramas (nuevos: list-add/remove, go-up/down, git-commit/merge/pull/push/stash, check-circle, studio-assistant, studio-advice); botón de commit primario con el acento; cajas redondeadas; estado vacío con icono; la caja del mensaje crece.
- Pruebas: la lógica pura en `git_smoke` (redacción, prompts, guía, limpieza, parser de salida) y los flujos con un `chxchx-tech` falso en `git_ui_smoke` (sin agente, sugerir, consejo, error, cancelar, cambiar de agente, conflictos). 11 tests nativos pasan; 10+15 ejecuciones seguidas del UI sin fallos; 4 mutaciones de privacidad (archivo sensible enviado, secretos sin enmascarar, diff sin enmascarar, sin `--lean`) detectadas.
- Bugs encontrados al probar: arranque con fallo de segmentación (el asistente se configuraba antes de crear el cliente del bridge) y una carrera (el consejo de conflictos no sabía si la fusión estaba a medias): corregidos.
- Sin probar con Codex/Claude/OpenCode reales (flags efímeros, borrado de sesión) ni con el consentimiento en un escritorio.

## Visor de Git en Studio (fase 2: enviar/recibir, stash y explorador)

- Motor: `git_repository_remote.cpp` (`fetch`, `pull` ff-only o merge, `push`/publicar, `mergeInProgress`/`abortMerge`, stash), `GitProblem` (credenciales, rechazado, ramas separadas, sin upstream, sin remoto, identidad, cancelado), cancelación de operaciones lentas, procesos en sesión nueva sin terminal. `git_explorer` (estado por ruta) y `GitTreeDelegate` (color, letra y punto en carpetas). Panel: botón de sincronización con acción inteligente, confirmación de envío (`git/confirmPush`), diálogo al separarse las ramas y sección Apartados.
- Pruebas: `git_smoke` crece con dos clones y un remoto bare (publicar, recibir, enviar, divergencia, rechazo, conflicto con abandono, sin remoto, cancelación, stash); `git_ui_smoke` cubre los flujos del panel. 11 tests nativos pasan; 20 ejecuciones seguidas sin fallos; 5 mutaciones (publicar sin `-u`, pull sin ff-only, push forzado, cancelar sin efecto, rechazo sin reconocer) detectadas.
- Gancho de QA: `CHXCHX_STUDIO_SCREENSHOT_TOOL=git:files` muestra el explorador. Capturas: `studio-git-sync.png`, `studio-git-explorer.png`.
- Sin probar contra un remoto real con autenticación (SSH/HTTPS), ni en Windows o macOS. Decisión: sin rebase ni envío forzado.

## Estado actual

- Producto: control plane local con CLI/TUI en Python y ChxChx Studio en C++20/Qt 6 Widgets con Scintilla/Lexilla. Studio llama al CLI mediante el bridge JSON v1 (`workspace/bridge_contracts.py`); Python sigue siendo la autoridad. La rama de integración es `dev`.
- Studio cubre todas las áreas de la TUI (inventario en `.ai/EDITOR-PARITY.md`). Toda escritura pasa por `--dry-run`, preview y confirmación. Inicio ofrece `Preparar y entrar`. Incluye terminal PTY/ConPTY + libvterm (scrollback de 5000 líneas), pestañas de agentes Codex/Claude/OpenCode (+Ollama), búsqueda de rutas y contenido, archivos recientes y grupos divididos, diff Git por archivo, símbolos básicos, atajos configurables y paleta `Ctrl+P`.
- Catálogo: 21 skills, 13 Tech Packs y 9 Mods de Claude Code en `.claude/skills`. RAM Governor: `max_agents` 2, aviso RAM 70 %, crítico 85 % y aviso swap 40 %; trust local obligatorio antes de ejecutar procesos.
- CI/CD de proyectos del usuario: CI opt-in para Python/Node/CMake y CD opt-in a VPS con Docker Compose (ADR 0007), ambos en CLI/TUI/Studio.
- Distribución: CPack genera ZIP/DMG/TGZ y un `Setup.exe` NSIS. Stable `0.5.0` se instaló y verificó por versión en Fedora x86_64; no se lanzó la ventana. No se ha probado la instalación en equipos limpios ni en Windows/macOS.
- Memoria: `.ai/memory/PROJECT_MEMORY.md` guarda las peticiones explícitas, y Basic Memory se usa limitado al `memory_project` del proyecto. Al cerrar cada sesión de Codex/Claude se escribe un resumen en `.ai/history/SESSIONS.md`, que se inyecta en el siguiente chat nuevo. `CURRENT_STATE`/`HANDOFF` se compactan solos hacia `.ai/history/`.
- Lo que bloquea el uso diario: QA interactivo de Studio en Fedora con un proyecto real. El orden de trabajo está en `.ai/ROADMAP.md` (plan de noviembre 2026).
- Equipo actual: Fedora 44 con CMake 4.3, GCC 16, Qt 6.11 y Ninja. Studio stable `0.5.0` está en `~/.local/opt/chxchx-studio/stable-0.5.0`; el launcher `~/.local/bin/chxchx-studio` y la entrada de menú están instalados. La antigua instalación compilada de `dev` (`5eae006`) y sus accesos se retiraron el 2026-11-04. El estado anterior completo está en `.ai/history/CURRENT_STATE.md`.

## En progreso

- Falta que el usuario reinicie la TUI y pruebe `A`. Una pestaña `Agentes` conservada de un layout viejo debe recrearse con `chxchx-tech run . --recreate`.
- En el primer inicio de Claude Code, aprobar la carpeta desde su pane si presenta su propio diálogo.
