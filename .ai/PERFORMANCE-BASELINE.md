# Baseline de rendimiento del legado

- Medido: 2026-11-12, Fedora 44 x86_64; Ryzen 5 3500U (4C/8T), 21 GiB RAM.
- Release del Studio: GCC 16.2.1, CMake 4.3.0, Ninja 1.13.2, Qt 6.11.2.
- Los resultados son locales y con caché de filesystem cálida; no comparan Rust.

## Fixtures

`scripts/perf/create_fixtures.py /tmp/chxchx-perf-baseline.MNI0jw/fixtures`
creó dos árboles deterministas que no existían antes. El pequeño tiene 21
archivos indexables; el grande, 5,001. Ambos incluyen directorios `.git`,
`target` y `node_modules` con sentinelas para comprobar que se excluyen.

## Medidas

| Operación | Fixture pequeño | Fixture grande |
| --- | ---: | ---: |
| CLI Python `status PATH`, startup p50/p95 | 320.9 / 381.0 ms | 339.9 / 372.1 ms |
| Studio ventana offscreen, proxy de startup p50/p95 | 302.2 / 380.0 ms | 303.9 / 330.4 ms |
| Studio RSS idle mediana / máximo | 65.7 / 65.7 MiB | 67.0 / 67.1 MiB |
| Studio CPU idle en muestra | 0.0 % | 0.0 % |
| Construcción del índice p50/p95 | 0.39 / 0.59 ms | 140.8 / 172.0 ms |
| Búsqueda de rutas, media por 100 llamadas p50/p95 | 0.032 / 0.052 ms | 7.48 / 9.18 ms |
| Búsqueda de contenido sin coincidencia p50/p95 | 0.91 / 1.08 ms | 118.4 / 135.5 ms |

Startup Python: 30 lanzamientos por fixture, entorno aislado con `CHXCHX_TECH_HOME`.
Studio: 30 lanzamientos por fixture; tiempo de ejecución menos un retardo fijo
de captura de 500 ms. Es un proxy de ventana lista, no una medida instrumentada
del primer frame. La app y RSS/CPU se midieron offscreen; RSS usa 5 s de warmup
y diez muestras de 1 s. CPU es tiempo del proceso durante esas diez muestras.
El caché de filesystem estaba cálido después de la primera pasada. No se midió
un escenario activo de edición/búsqueda ni memoria de otro proceso hijo.

## Tamaños y validez del paquete

- Wheel Python `chxchx_tech_lead-0.5.4-py3-none-any.whl`: 361,235 bytes.
- Ejecutable Release `chxchx-studio`: 9,106,008 bytes.
- TGZ de CPack `chxchx-studio-0.5.4-Linux-x86_64.tar.gz`: 4,312,761 bytes.
- El TGZ no es un paquete válido/portable en este Fedora: `scripts/verify_studio_package.py`
  reporta que faltan `libqt6core.so.6`, `libqt6widgets.so.6`,
  `libqt6core5compat.so.6` y `platforms/libqxcb.so`. `cmake --install` tampoco
  copió esas dependencias. El tamaño se registra como salida actual, no como
  baseline aceptable; R0.4 no cierra hasta medir un paquete verificable. El
  deploy genérico Qt omite bibliotecas de `/lib64`; forzarlas y pedir xcb
  también falla al reajustar el RPATH de un plugin Fedora sin entrada RPATH.
  Ese intento se revirtió, sin dejar una modificación experimental.

## Presupuestos provisionales para comparar el primer prototipo Rust

Aplicar en el mismo equipo, fixtures y condiciones; no son SLO de release:

- CLI: p95 de status en ambos árboles <= 200 ms.
- Studio: p95 del mismo proxy <= 350 ms en ambos árboles.
- Studio idle: RSS máximo <= 80 MiB y CPU <= 1 %.
- Índice grande p95 <= 200 ms; path search (media por 100 llamadas) p95 <=
  10 ms; content search p95 <= 150 ms.
- No fijar todavía un límite de paquete Studio: primero corregir el TGZ y
  verificarlo. Comparar el paquete Rust tras elegir distribución/runtime.

## Reproducción

```bash
python scripts/perf/create_fixtures.py /tmp/chxchx-perf-baseline.MNI0jw/fixtures
env -u LD_LIBRARY_PATH cmake -S native -B /tmp/chxchx-native-baseline.NzU66P -G Ninja -DCMAKE_BUILD_TYPE=Release -DCHXCHX_BUILD_TESTS=ON -DCHXCHX_BUILD_BENCHMARKS=ON
env -u LD_LIBRARY_PATH cmake --build /tmp/chxchx-native-baseline.NzU66P --parallel 2
env -u LD_LIBRARY_PATH /tmp/chxchx-native-baseline.NzU66P/chxchx-project-file-index-bench /tmp/chxchx-perf-baseline.MNI0jw/fixtures/small 30
env -u LD_LIBRARY_PATH /tmp/chxchx-native-baseline.NzU66P/chxchx-project-file-index-bench /tmp/chxchx-perf-baseline.MNI0jw/fixtures/large 30
env -u LD_LIBRARY_PATH uv run python scripts/perf/measure_processes.py --small /tmp/chxchx-perf-baseline.MNI0jw/fixtures/small --large /tmp/chxchx-perf-baseline.MNI0jw/fixtures/large --studio /tmp/chxchx-native-baseline.NzU66P/chxchx-studio --runs 30 --idle-samples 10 --screenshot-delay-ms 500
uv build --wheel --out-dir /tmp/chxchx-perf-baseline.MNI0jw/python-package
env -u LD_LIBRARY_PATH cpack --config /tmp/chxchx-native-baseline.NzU66P/CPackConfig.cmake -C Release -B /tmp/chxchx-perf-baseline.MNI0jw/packages
stat -c '%n %s bytes' /tmp/chxchx-perf-baseline.MNI0jw/packages/*.tar.gz /tmp/chxchx-perf-baseline.MNI0jw/python-package/*.whl
python scripts/verify_studio_package.py --platform linux-fedora-x86_64 --package-dir /tmp/chxchx-perf-baseline.MNI0jw/packages --output-dir /tmp/chxchx-perf-baseline.MNI0jw/staged-package
```

Las rutas se eligieron como temporales para esta corrida y deben estar vacías
al repetirla, ya que el generador de fixtures no sobrescribe árboles existentes.
