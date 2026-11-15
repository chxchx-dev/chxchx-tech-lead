# Memoria en ChxChx Studio: qué se hace y por qué

Guía para entender (y comprobar tú mismo) cómo Studio y los agentes que abre gastan la menor RAM posible.

## 1. Lo que importa medir

Un programa tiene varias «memorias». Para esto solo importa una:

| Nombre | Qué es | Dónde se ve |
|---|---|---|
| **Memoria virtual** | Espacio reservado, casi siempre sin usar | `VmSize` en `/proc/<pid>/status` (ignórala) |
| **Residente (RSS / working set)** | Lo que de verdad ocupa RAM ahora | `VmRSS`, Administrador de tareas, `top` |

Studio muestra su residente en la **barra de estado** (abajo a la derecha, junto a la versión). Se actualiza cada 5 s.

## 2. La idea central: liberar no es devolver

Cuando un programa libera memoria (`drop`, `free`), el *asignador* (el código que reparte memoria) normalmente **no la devuelve al sistema**: la guarda para reutilizarla. Es rápido, pero tras un momento pesado (abrir una foto grande, un chat con mucha salida) el número residente se queda alto aunque el programa ya no use esa memoria.

Por eso Studio hace dos cosas: usar menos y **devolver** lo que ya no usa.

## 3. Qué se hace, pieza por pieza

### a) Devolver memoria al sistema — `src/integrations/process_memory.rs`
- `give_back()` pide al asignador que entregue lo liberado: `malloc_trim(0)` en Linux (glibc) y recorte del *working set* en Windows (`SetProcessWorkingSetSize(-1, -1)`).
- Se llama **en momentos tranquilos**: cada 30 s (`shell::Event::Memory`), al cerrar una terminal, al terminar un turno de chat y tras decodificar una imagen. Siempre en un hilo aparte, para no frenar la interfaz.
- **Lección:** hacerlo en cada operación sería caro; hacerlo nunca deja la memoria «prestada». Una frecuencia baja más disparadores puntuales es el equilibrio.

### b) Programas hijos más austeros — `LEAN_ENV`
Todo lo que Studio lanza (el CLI del chat, shells, agentes en terminal) recibe:
- `MALLOC_ARENA_MAX=2`: glibc crea una «arena» de memoria por hilo; un agente con muchos hilos acaba con decenas de arenas a medio llenar. Con 2 se reduce el residente de forma notable. Es un ajuste clásico para procesos multihilo.
- `MALLOC_TRIM_THRESHOLD_=131072`: devuelve al sistema la memoria libre del final del heap en cuanto hay 128 KiB libres.

En otros sistemas esas variables simplemente se ignoran.

### c) El chat no acumula lo que nadie ve — `features/chat_model.rs`
- La salida de una herramienta se guarda **solo por su final** (máx. 8 000 caracteres): un `cat` de un archivo enorme no ocupa megas en el historial.
- Los mensajes antiguos (más allá de los últimos 60) **sueltan** su salida y su razonamiento, y su Markdown renderizado se descarta (`Model::compact`, `chat::sync`). Siguen contando como líneas en su sitio, así que nada se desordena.
- El chat solo *dibuja* los últimos 60: menos memoria y un scroll más fluido a la vez.

### d) Imágenes: lo mínimo que el modelo aprovecha — `integrations/chat_images.rs`
- Se reducen a 1568 px de lado mayor y se recomprimen; solo viven en disco hasta que termina el turno.
- Decodificar una foto de 20 MB necesita cientos de MB temporales; el resultado ocupa una fracción. Tras decodificar se llama a `give_back()`.
- En pantalla solo se guarda una miniatura de 120 px.

### e) Terminales
- Historial limitado (1 000 líneas por sesión).
- Las sesiones **ocultas** se leen cada 500 ms en vez de cada 100 ms, así no se reconstruye la ventana sin motivo.
- Cerrar una sesión libera su pantalla virtual y devuelve la memoria.

### f) Perfil de compilación
`lto = "thin"`, `codegen-units = 1` y `strip = "symbols"` en `Cargo.toml`: menos código y menos tablas de símbolos en memoria.

## 4. Cómo comprobarlo tú

```bash
# Residente de Studio ahora (Linux)
grep VmRSS /proc/$(pgrep -x chxchx-studio)/status

# Su evolución, cada 2 s
watch -n2 'grep -E "VmRSS|Threads" /proc/$(pgrep -x chxchx-studio)/status'

# Los agentes que lanzó (hijos)
ps --ppid $(pgrep -x chxchx-studio) -o pid,rss,cmd
```

Experimento sugerido: abre un chat, pega una captura grande, espera a que termine el turno y mira la barra de estado: sube al procesar y baja pocos segundos después. Compáralo quitando `give_back()` en `studio_chat_images.rs` y compilando.

Pruebas automáticas relacionadas: `process_memory::tests`, `chat_model_tests` (salida truncada y compactación).

## 5. Lo que NO se hace (y por qué)

- **No se limita la memoria de los agentes** (`NODE_OPTIONS=--max-old-space-size`…): un límite demasiado bajo rompe tareas legítimas. Solo se ajusta cómo piden y devuelven memoria.
- **No se usa GPU (`wgpu`)**: acelera el dibujo pero sube el uso de memoria y el tamaño del programa. Se valorará si el scroll lo necesita.
- **Un chat inactivo no consume procesos**: cada mensaje lanza el CLI, que termina al acabar el turno. Los agentes en *terminal* sí siguen vivos mientras la sesión esté abierta; ciérrala para liberarlos.

## 6. Dónde tocar si quieres seguir aprendiendo
| Quiero cambiar… | Archivo |
|---|---|
| Cada cuánto se devuelve memoria | `studio_shell.rs` (`MEMORY_TRIM_EVERY`) |
| Ajustes de los hijos | `integrations/process_memory.rs` (`LEAN_ENV`) |
| Cuántos mensajes se muestran | `studio_chat.rs` (`VISIBLE_ENTRIES`) |
| Tope de salida de herramientas | `features/chat_model.rs` (`MAX_OUTPUT`) |
| Tamaño máximo de imágenes | `integrations/chat_images.rs` (`MAX_SIDE`) |
