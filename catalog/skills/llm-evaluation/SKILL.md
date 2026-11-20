# Evaluación de modelos de lenguaje

Una mejora que no se midió bien no es una mejora. La evaluación debe poder repetirse y debe poder fallar.

## Qué evaluar

- **Pérdida o perplejidad en un conjunto retenido** del dominio real: la señal más estable durante el
  entrenamiento.
- **Tareas del producto:** casos escritos a partir del uso real (preguntas, entradas típicas y difíciles),
  con la respuesta esperada o un criterio verificable.
- **Benchmarks públicos** para comparar con otros, sabiendo que pueden estar contaminados.
- **Seguridad y robustez:** rechazos apropiados, inyección de instrucciones, entradas ruidosas, otros idiomas.

## Protocolo

1. Congela el conjunto de evaluación, la plantilla del prompt, el número de ejemplos en contexto y los
   parámetros de decodificación. Cualquier cambio invalida la comparación.
2. Usa decodificación determinista (temperatura 0) para métricas de exactitud; si muestreas, repite con varias
   semillas y reporta la media con su intervalo de confianza.
3. Compara siempre contra una **línea base** (el modelo anterior o un modelo de referencia) con el mismo
   protocolo. Un número aislado no significa nada.
4. Registra el modelo exacto, el tokenizador, la versión del código, los datos y la configuración.

## Contaminación y trampas

- Comprueba que los ejemplos de evaluación no aparecen en el entrenamiento (coincidencia de n-gramas largos).
  Si hay solapamiento, separa los resultados con y sin los ejemplos afectados.
- No ajustes hiperparámetros contra el conjunto de prueba: usa validación para decidir y la prueba una vez.
- Cuidado con el formato: muchas diferencias entre modelos son del extractor de la respuesta, no del modelo.
  Verifica a mano una muestra de aciertos y fallos.
- **Modelo como juez:** úsalo con una rúbrica concreta, con el orden de las respuestas aleatorizado y
  calibrado contra juicio humano en una muestra; reconoce sus sesgos (longitud, estilo propio).

## Estadística mínima

- Con n ejemplos, el error estándar de una exactitud p es √(p(1−p)/n). Con 100 ejemplos, una diferencia de
  3 puntos casi nunca es real. Usa pruebas pareadas (bootstrap o McNemar) para comparar dos modelos.
- Reporta tamaño de la muestra y dispersión junto a cada métrica.

## Compuertas de regresión

- Define umbrales: qué métricas no pueden bajar más de cuánto para aceptar un cambio.
- Automatiza la evaluación en integración continua con un subconjunto rápido, y deja la completa para antes de
  publicar. Un fallo de compuerta bloquea la entrega y se explica, no se silencia.
- Guarda los resultados por versión para ver la tendencia y poder reproducir cualquier cifra publicada.
