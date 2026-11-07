Pasa `context.Context` como primer argumento en operaciones que bloquean y respeta su cancelación. Devuelve errores envueltos con `%w` y contexto útil; no los ignores. Cada goroutine debe tener una forma clara de terminar, y los canales y mutex deben tener un dueño evidente.

Mantén interfaces pequeñas definidas donde se consumen, usa pruebas por tabla y ejecuta `go vet`, `go test -race` y `gofmt` antes de cerrar. Evita dependencias externas cuando la biblioteca estándar basta.
