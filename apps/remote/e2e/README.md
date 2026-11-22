# Pruebas de navegador del cliente web

Arrancan un nodo real (con el cliente dentro), un proyecto de confianza y un agente falso, y
manejan la app en Chromium como lo haría una persona: vincular con un código o un QR, escribir a un
agente, aprobar o rechazar un permiso, responder una pregunta, cancelar, roles, desvincular un
dispositivo en vivo, la caída y vuelta del nodo, el arranque sin red y la accesibilidad básica.
`perf.mjs` mide la carga con una red 4G simulada.

```bash
scripts/build-remote.sh && cargo build --features node --bin chxchx-tech   # desde la raíz
cd apps/remote/e2e && npm install
CHXCHX_BIN=../../../target/debug/chxchx-tech node e2e.mjs
```

- Necesita Node y un Chromium: `npx playwright-core install chromium`, o el que ya tengas con
  `CHROME_PATH=/ruta/a/chrome`.
- El entorno vive en `world/` (carpeta ignorada) y usa el puerto 47831 y un `CHXCHX_TECH_HOME`
  propio: no toca tu estado. Si algo falla queda el nodo en marcha; `./env.sh stop` lo detiene.
- Las capturas de cada pantalla quedan en `shots/`.
- Lo que **no** cubre: Safari/iOS, Firefox ni un teléfono real; la tailnet (se prueba con
  `chxchx-tech node remote`, ver `.ai/NODE-SECURITY.md`) y las notificaciones.
