No optimices la inferencia a ciegas: mide primero tiempo hasta el primer token, tokens por segundo y rendimiento bajo carga con prompts de longitud real. Cambia una cosa a la vez y repite la medición.

Toda cuantización o conversión se valida contra el modelo original con las mismas tareas y perplejidad; acéptala solo dentro del umbral acordado. Calcula la memoria (pesos y KV cache) antes de elegir contexto y concurrencia, y limita tamaño de entrada, tokens generados y tiempo por petición.
