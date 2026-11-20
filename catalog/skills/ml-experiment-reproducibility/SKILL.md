# Experimentos reproducibles

Un resultado que nadie puede repetir no sirve para decidir. Haz que cada corrida deje todo lo necesario.

## Qué debe quedar registrado

- **Código:** commit exacto y si había cambios sin confirmar (en ese caso, el diff).
- **Configuración completa:** todos los hiperparámetros, incluidos los valores por defecto implícitos.
- **Datos:** versión y hash del conjunto, y de los fragmentos de validación.
- **Entorno:** versión de Python, de las bibliotecas (archivo de bloqueo), del controlador y de CUDA, y el
  hardware.
- **Semillas** de todos los generadores y si el modo determinista estaba activo.
- **Resultados:** curvas, métricas finales y de evaluación, y los artefactos (checkpoints, tokenizador).

## Práctica

- Una sola fuente de configuración (archivo versionado + sobrescrituras de línea de comandos que también se
  registran). Nada de constantes escondidas en el código.
- Fija las dependencias con un bloqueo (`uv.lock`, `pip-tools`) y construye el entorno desde él; para GPU,
  fija también la imagen del contenedor.
- Usa un seguimiento de experimentos (MLflow, Weights & Biases o archivos locales) con nombres de corrida
  significativos y etiquetas (objetivo, versión de datos, línea base).
- Guarda los artefactos con un hash; no sobrescribas resultados: cada corrida tiene su identificador.
- Prueba la reproducibilidad: repite una corrida corta con la misma semilla y confirma que la curva coincide
  (o que la diferencia es la esperada por no determinismo).

## Comparar con honestidad

- Compara contra una línea base con el mismo protocolo de evaluación y varias semillas; reporta la media con su
  dispersión. Una sola corrida favorable no es una mejora.
- Cambia una variable por experimento o usa un diseño que permita atribuir el efecto.
- Anota los experimentos que fallaron o no mejoraron: evitan repetirlos.

## Ficha de corrida (run card)

Al terminar, resume en un archivo: objetivo, qué cambió respecto a la línea base, configuración, datos,
resultados, conclusiones, problemas conocidos y cómo repetirla con un solo comando.

## Reanudación y costos

- Toda corrida larga debe poder reanudarse (ver la skill de entrenamiento) y debe dejar registro de su costo
  (GPU-horas) para decidir si vale la pena seguir.
