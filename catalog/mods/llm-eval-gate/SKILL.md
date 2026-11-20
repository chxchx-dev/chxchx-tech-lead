No declares que un modelo, un prompt o una configuración mejora algo sin medirlo contra una línea base con el mismo protocolo: conjunto de evaluación y plantilla congelados, decodificación fija, varias semillas e intervalo de confianza.

Comprueba que los ejemplos de evaluación no estén en el entrenamiento y no ajustes hiperparámetros contra el conjunto de prueba. Reporta exactamente qué se ejecutó y los resultados, incluidos los que empeoraron, y bloquea el cambio si una métrica protegida cae por debajo de su umbral.
