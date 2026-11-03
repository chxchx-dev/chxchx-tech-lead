# Pruebas

Cuatro capas, de la más barata a la más real. Las tres primeras corren sin instalar nada y en CI.

| Capa | Dónde | Qué prueba | Cómo se corre |
| --- | --- | --- | --- |
| Unitarias | `tests/**/test_*.py` | Lógica pura (parsers, encaje de modelos, fusión de `opencode.json`) con stubs en memoria. | `uv run pytest` |
| E2E con falsos | `tests/e2e/` | Flujos completos con **HTTP real** y **procesos reales**: un servidor falso de Ollama y ejecutables falsos de `ollama` y `opencode` en el PATH. Ejercitan stdin, argv, entorno, códigos de salida y JSONL tal como en producción. | `uv run pytest` |
| Nativas | `native/tests/` | Studio (Qt): parseo, estados de la pestaña, chat. | `ctest` (ver abajo) |
| Live | `tests/live/` | Las herramientas **reales**. Confirman las suposiciones que los falsos solo imitan. | `uv run pytest -m live -v -s` |

`uv run pytest` **excluye** `live` (`addopts = "-m 'not live'"`). Con `-m live` y sin las herramientas instaladas, todo se salta con el motivo.

## Los falsos

- `tests/fakes/fake_ollama.py`: servidor HTTP en un puerto libre con `/api/version`, `/api/tags`, `/api/ps`, `/api/pull` (NDJSON con progreso), `/api/create` y `DELETE /api/delete`. Guarda estado y registra cada petición. Fixture `fake_ollama` (apunta `OLLAMA_HOST` a él).
- `tests/fakes/fake_clis.py`: `ollama` (su `create` habla con el servidor falso, como el real) y `opencode` (`run`, `session list`, `export`, `models`). Fixture `fake_bin` los pone en el PATH y devuelve una función que lee lo que recibieron (`argv`, `stdin`, `OPENCODE_PERMISSION`, `cwd`).
- `isolated_home` aísla el estado de ChxChx, Codex, Claude y XDG bajo `tmp_path`.

Un falso solo vale si las pruebas fallan cuando el código se rompe. Al añadir una, **muta el código** (cambia `plan`/`build`, quita el `--yes`, invierte el orden de borrado…) y comprueba que falla.

## Cómo probarlo tú con las herramientas reales

### 1. Sin instalar nada
```bash
uv run pytest                                  # unitarias + e2e con falsos
```

### 2. OpenCode real
```bash
npm install -g opencode-ai
opencode auth login                            # o configura un proveedor / un modelo local
uv run pytest -m live -v -s tests/live/test_live_opencode.py
# Opcional: fijar el modelo y guardar la salida cruda para renovar fixtures
CHXCHX_LIVE_OPENCODE_MODEL=anthropic/claude-sonnet-5-5 CHXCHX_LIVE_CAPTURE=/tmp/oc uv run pytest -m live -v -s tests/live/test_live_opencode.py
```
Cada test dice qué suposición confirma:

| Test | Si falla, revisa |
| --- | --- |
| `prompt_on_stdin…` | Si el prompt por stdin no funciona, pásalo como argumento posicional en `turn_command`. Si cambian los campos de los eventos, `agent_stream_opencode.py`. |
| `a_session_can_be_resumed…` | `--session` en `turn_command` y el `sessionID` del evento `step_start`. |
| `tool_calls_arrive…` | La forma de `tool_use` (`part.tool`, `part.callID`, `part.state`). |
| `read_only_mode_really_blocks…` | `OPENCODE_PERMISSION` y `--agent plan`. **Es el de seguridad: si falla, el modo solo lectura no protege.** |
| `workspace_write_mode_can_edit` | Que `--agent build` sin `--auto` pueda escribir sin pedir permiso. |
| `an_ask_permission_without_a_terminal…` | Caracterización: no falla por su resultado; con `-s` imprime si el permiso se rechazó o se concedió. |

### 3. Ollama real
```bash
# instala Ollama (https://ollama.com/download) y deja el servidor activo
uv run pytest -m live -v -s tests/live/test_live_ollama.py          # estado, /api/ps, borrado inexistente

# Ciclo completo: descarga, variante -ctx32k, opencode.json, borrado. Descarga ~2 GB.
CHXCHX_LIVE_PULL=qwen2.5-coder:3b uv run pytest -m live -v -s tests/live/test_live_ollama.py
```
El test limpia lo que creó y **no borra** un modelo que ya tuvieras (solo la variante).

### 4. Recorrido manual en Studio
```bash
./scripts/dev-studio.sh /ruta/a/un/proyecto-de-prueba
```
1. `Inicio → Modelos locales` (o `Herramientas → Modelos locales…`). Sin Ollama: aviso con el comando oficial. Con Ollama detenido: aviso y Quitar/Descargar desactivados.
2. Con Ollama activo: tabla con encaje y ★. Elige un modelo pequeño → **Vista previa** (no cambia nada).
3. **Descargar y configurar** → confirma, mira la barra de progreso, prueba **Cancelar** y vuelve a lanzarlo (se reanuda).
4. Revisa `opencode.json` del proyecto y `~/.chxchx-tech-lead/backups-opencode/`.
5. Abre el chat visual con el agente OpenCode (`chxchx-tech agent add opencode`), elige `ollama/<modelo>-ctx32k` y pídele que lea un archivo.
6. **Quitar** el modelo: desaparece de la tabla, de `ollama list` y de `opencode.json`.

La misma secuencia sin Studio: `chxchx-tech local-models status|setup|remove` (todos con `--dry-run`).

### 5. Nativas
```bash
cmake -S native -B /tmp/chxchx-native -DCHXCHX_BUILD_TESTS=ON
cmake --build /tmp/chxchx-native --parallel
QT_QPA_PLATFORM=offscreen ctest --test-dir /tmp/chxchx-native --output-on-failure
# Captura de la pestaña de modelos locales para revisarla a ojo:
CHXCHX_LOCAL_MODELS_SCREENSHOT=/tmp/modelos.png QT_QPA_PLATFORM=offscreen /tmp/chxchx-native/chxchx-local-models-smoke
```

## Capturas para el README

Studio puede guardar una captura de su ventana y salir (ayuda de desarrollo): `CHXCHX_STUDIO_SCREENSHOT=ruta.png`, con `CHXCHX_STUDIO_SCREENSHOT_SIZE=1280x800`, `…_AREA=overview` para elegir sección, `…_DELAY_MS=5000` para esperar datos y `…_TOOL=` para abrir una herramienta: `local-models`, `add-agent` (selecciona la entrada de añadir OpenCode) o `chat:AGENTE|mensaje`.

Las capturas de `assets/screenshots/` se hicieron con un proyecto de ejemplo y un `CHXCHX_TECH_CLI` de demostración (un script que devuelve datos inventados para `bridge local-models`, `bridge agent-options` y `agent chat`, y delega lo demás en el CLI real), así no aparecen rutas, equipo ni nombres reales. Revisa cada imagen antes de publicarla.

## Reglas
- Un módulo de producción no pasa de 300 líneas (`test_code_size.py`).
- Los tests no tocan tu `$HOME`: usa `isolated_home`/`tmp_path`.
- Las pruebas live nunca descargan ni borran nada sin una variable explícita, y nunca borran lo que ya existía.
