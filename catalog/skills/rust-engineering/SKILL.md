# Ingeniería con Rust

## Errores y APIs

- Devuelve `Result` con errores tipados y con contexto; reserva `panic!`, `unwrap` y `expect` para invariantes que
  de verdad no pueden romperse y explícalas. En bibliotecas, un tipo de error propio (`thiserror`); en
  aplicaciones, contexto con `anyhow`.
- Prefiere tipos que hagan imposibles los estados inválidos (nuevos tipos, enums en vez de banderas,
  constructores que validan).
- Recibe `&str`, `&[T]` y `impl AsRef<Path>`; devuelve tipos propios cuando la llamada debe poseer el dato.
  No clones para silenciar al compilador sin entender por qué falla.

## Propiedad y concurrencia

- Diseña para la propiedad antes de pelear con préstamos: estructuras pequeñas, datos inmutables y paso de
  mensajes. `Arc<Mutex<_>>` en todas partes es una señal de diseño.
- No mantengas un cerrojo (`Mutex`) durante `await` ni durante E/S. En código asíncrono, no bloquees el
  ejecutor: usa `spawn_blocking` para trabajo bloqueante o intensivo.
- Cancelación: define qué pasa si un futuro se descarta a medias y deja el estado consistente.

## `unsafe`

- Mínimo, encapsulado en un módulo pequeño y con un comentario `// SAFETY:` que explique el invariante que lo
  hace correcto. Cada bloque `unsafe` lleva pruebas, y Miri o sanitizadores cuando sea posible.

## Dependencias y compilación

- Fija dependencias con `Cargo.lock` en binarios y usa `cargo audit` o `cargo deny` para vulnerabilidades y
  licencias. Revisa lo que cada dependencia añade (tiempo de compilación, `unsafe`, mantenimiento).
- Activa solo las características necesarias; mantén una configuración reproducible (`--locked`).

## Verificación antes de entregar

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

- Pruebas de unidad junto al código, de integración en `tests/` y pruebas de propiedades (`proptest`) donde hay
  invariantes. Cubre los errores, no solo el camino feliz.
- Comprueba que compila en todas las plataformas objetivo cuando el código depende del sistema (rutas,
  procesos, finales de línea).
