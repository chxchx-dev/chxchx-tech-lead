# agent-adapter

Servicio que conduce Codex (`codex app-server`) y OpenCode (`opencode serve`) en su modo en vivo para el
producto Rust de ChxChx. Solo usa la biblioteca estándar de Python (3.11+).

**Rust decide, el servicio ejecuta.** Rust conserva trust, configuración, nivel de acceso, contexto y transcript.
Le pasa al servicio una solicitud con el proceso exacto que debe lanzar; el servicio habla el protocolo del
proveedor, traduce lo que imprime a eventos `chxchx.agent-event` v1 y reenvía las respuestas de la persona.
No lee archivos de proyecto ni ajustes de ChxChx.

## Contrato (versión 1)

El contrato completo está documentado en [`contract.py`](src/chxchx_agent_adapter/contract.py) y lo valida
[`tests/test_contract.py`](tests/test_contract.py). En resumen:

```text
chxchx-agent-adapter contract     → {"schema":"chxchx.agent-adapter","contract_version":1,"version":…,"providers":[…]}
chxchx-agent-adapter run          stdin  línea 1: solicitud (JSON)
                                         líneas siguientes: {"type":"answer",…} | cancel
                                  stdout eventos chxchx.agent-event v1, y al final un `adapter_done`
```

- El fin de la entrada también detiene el turno: nada queda ejecutándose si el anfitrión desaparece.
- El servicio solo ejecuta `codex app-server` o `opencode serve`; cualquier otro `argv` se rechaza.
- Un cambio incompatible sube `contract_version`; el adapter Rust (`src/integrations/agent_adapter.rs`) no usa un
  servicio que hable otra versión.

## Desarrollo

```bash
cd services/agent-adapter
uv run --with pytest python -m pytest      # o: python -m pip install pytest && python -m pytest
```

Los tests usan `codex` y `opencode serve` falsos (`tests/fake_clis.py`) que son procesos reales encontrados por
`PATH`. El lado Rust tiene su propio test de extremo a extremo con este servicio
(`the_real_service_drives_a_fake_codex_for_a_rust_turn`).

## Cómo lo encuentra Rust

En orden: `CHXCHX_AGENT_ADAPTER`, junto al binario (instalado), el entorno de un checkout
(`services/agent-adapter/.venv/bin/chxchx-agent-adapter`) y el `PATH`. Antes de usarlo ejecuta `contract`.
