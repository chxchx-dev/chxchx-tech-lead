# iced_tiny_skia (copia con un cambio)

Copia de `iced_tiny_skia` 0.14.1 (MIT, proyecto iced: https://github.com/iced-rs/iced).

Único cambio, marcado como `ChxChx patch` en `src/engine.rs`: un cuadrado liso (un color, sin radio, borde ni
sombra) se dibuja como rectángulo y no como un camino con antialiasing y máscara. Era lo que dominaba cada
fotograma al hacer scroll. Al subir iced, vuelve a copiar la versión nueva y reaplica el cambio (o descártalo si ya
lo incluye).

Otros cambios: `adjust_clip_mask` reescribe la máscara solo donde cambia; cada cuadro se dibuja completo (en
`window/compositor.rs`): reutilizar búferes viejos dejaba letras acumuladas al hacer scroll en Wayland;
`CHXCHX_PARTIAL_REDRAW=1` devuelve el comportamiento original; pruebas `chxchx_scroll_tests` simulan scroll con el cálculo de daño de la ventana.
