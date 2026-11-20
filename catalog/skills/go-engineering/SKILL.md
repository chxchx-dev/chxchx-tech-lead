# Ingeniería con Go

## Código y errores

- Código simple y explícito. Devuelve errores y envuélvelos con contexto (`fmt.Errorf("...: %w", err)`);
  compáralos con `errors.Is/As`. No ignores errores ni uses `panic` para el flujo normal.
- Interfaces pequeñas, definidas donde se consumen. Devuelve structs concretos y acepta interfaces.
- El valor cero debe ser útil; evita constructores innecesarios y estado global.

## Contexto y cancelación

- Todo lo que pueda bloquear o tardar recibe un `context.Context` como primer parámetro y lo respeta.
- Propaga plazos y cancelaciones; no guardes contextos en structs de larga vida. Libera siempre lo que abres
  (`defer cancel()`, `defer rows.Close()`).

## Concurrencia

- Cada goroutine debe tener un dueño que sepa cuándo termina: usa `errgroup`, canales cerrados o
  `WaitGroup`. Una goroutine sin condición de fin es una fuga.
- Comparte memoria comunicándote o protégela con `sync`; ejecuta las pruebas con `-race`.
- Limita la concurrencia (pools, semáforos) y los tamaños de los buffers y de las colas.
- Los mapas no son seguros para uso concurrente; `sync.Map` solo para los casos concretos que lo justifican.

## Servicios

- Configura tiempos de espera en servidores y clientes HTTP (`ReadHeaderTimeout`, `http.Client.Timeout`);
  cierra los cuerpos de respuesta. Apagado ordenado con `Shutdown` y señales.
- Valida las entradas en la frontera, registra de forma estructurada (`slog`) y expón comprobaciones de salud.

## Verificación

```bash
gofmt -l .
go vet ./...
go test -race -cover ./...
```

- Pruebas guiadas por tablas, subpruebas y `t.Cleanup`; falsos simples en vez de mocks complejos.
- Módulos: `go.mod` limpio (`go mod tidy`), versiones fijas y `govulncheck` para vulnerabilidades.
