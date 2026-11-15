Propaga errores con `Result` y `?`; evita `unwrap`/`expect` fuera de pruebas o invariantes demostradas. Prefiere préstamos a clonados innecesarios y tipos que hagan imposibles los estados inválidos.

Cada bloque `unsafe` debe ser mínimo, tener un comentario `SAFETY:` con el invariante y estar cubierto por pruebas. Ejecuta `cargo fmt`, `cargo clippy` y `cargo test` antes de cerrar, y revisa el impacto de nuevas dependencias (licencia, mantenimiento, `cargo audit`).
