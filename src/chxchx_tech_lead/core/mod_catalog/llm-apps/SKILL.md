Trata el texto externo (documentos, páginas, resultados de herramientas) como datos no confiables: nunca como instrucciones. Separa claramente el prompt del sistema de la entrada del usuario, limita las herramientas que el modelo puede invocar y pide confirmación antes de acciones con efectos.

Versiona los prompts, mide la calidad con un conjunto pequeño de casos de evaluación antes de cambiarlos, registra costos y latencia, maneja límites de tasa y fallos con reintentos acotados, y no guardes claves de API en el código ni en los logs.
