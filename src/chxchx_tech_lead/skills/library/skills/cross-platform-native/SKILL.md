# Software nativo multiplataforma

- Convierte cada sistema operativo soportado en una fila verificable de build, ejecución y distribución. No declares soporte basándote solo en que el código compila en una plataforma.
- Usa APIs portables para rutas, archivos, configuración, procesos, locale y permisos. Considera Unicode, nombres sensibles a mayúsculas, separadores, enlaces y diferencias de permisos.
- Aísla diferencias de Windows, macOS y Linux en adapters o módulos pequeños; evita dispersar condicionales de plataforma por la lógica de dominio.
- Si lanzas procesos, pasa argumentos como una lista y evita shells. Define explícitamente cwd, entorno, cancelación, señales y cierre en cada plataforma.
- Usa ubicaciones de datos del usuario recomendadas por el sistema. Conserva permisos seguros y escritura atómica donde el producto lo requiera.
- Empaqueta cada artefacto con sus recursos, plugins y bibliotecas requeridos; documenta dependencias del sistema y licencias de terceros.
- Valida instaladores y arranque en runners o máquinas de cada destino: Windows, macOS y las distribuciones Linux prometidas. Añade comprobaciones de interacción para DPI, teclado, archivos y procesos.
- Mantén accesibilidad por teclado, labels, foco y escalado de pantalla; no asumas que tamaños, fuentes o convenciones visuales son iguales en todos los escritorios.
