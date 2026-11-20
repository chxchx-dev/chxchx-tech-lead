# Entrenar un LLM desde cero

Un entrenamiento largo es caro y difícil de repetir. Antes de lanzarlo, demuestra con corridas pequeñas que
cada pieza funciona y que sabes qué resultado esperas.

## 1. Define el alcance con números

- Fija el presupuesto de cómputo (GPU-horas) y el objetivo (capacidad, idioma, dominio, tamaño máximo para
  servirlo). De ahí salen los parámetros y los tokens, no al revés.
- Como regla de partida, entrena con unos 20 tokens por parámetro (relación tipo Chinchilla); si el modelo
  se va a servir mucho, vale la pena sobreentrenar uno más pequeño. Anota la decisión y por qué.
- Calcula antes la memoria: parámetros × bytes por parámetro, más gradientes, estados del optimizador
  (AdamW guarda dos por parámetro, en fp32) y activaciones. Si no cabe, decide ahora entre acumulación de
  gradientes, checkpointing de activaciones o sharding (FSDP/ZeRO), no cuando falle a mitad de corrida.

## 2. Arquitectura: empieza por lo estándar

- Decoder-only con pre-norm (RMSNorm), posiciones rotatorias (RoPE), MLP con SwiGLU y atención con
  grupos de consultas (GQA) es una base probada. Cambia una cosa a la vez y mide su efecto.
- Elige dimensiones que alineen con el hardware (múltiplos de 64 o 128 para dimensión oculta, cabezas y
  vocabulario) y fija el contexto que de verdad entrenarás; extenderlo después cuesta una fase aparte.
- Inicializa con desviación pequeña (≈0,02) y escala las proyecciones de salida de cada bloque por
  1/√(2·capas). No enlaces ni dejes sin inicializar los pesos por accidente.

## 3. Comprobaciones antes de la corrida larga

1. **Pérdida inicial:** con pesos aleatorios debe rondar ln(tamaño del vocabulario). Si no, hay un error de
   inicialización, de etiquetas desplazadas o de máscara.
2. **Sobreajusta un lote pequeño:** la pérdida debe bajar casi a cero. Si no puede memorizar 8 ejemplos,
   no aprenderá nada más grande.
3. **Causalidad:** cambia un token futuro y confirma que no cambian los logits de las posiciones anteriores.
4. **Equivalencia:** el mismo resultado (dentro de tolerancia) con un solo dispositivo y con varios, y con y
   sin acumulación de gradientes.
5. **Reanudación:** corta una corrida y reanúdala desde el checkpoint; la curva debe continuar sin salto.
6. **Rendimiento:** mide tokens/s y el uso de la GPU (MFU). Una corrida al 20 % de MFU suele tener un
   problema de datos, de kernels o de comunicación que conviene resolver antes.

## 4. Optimización

- AdamW con β₂ ≈ 0,95, decaimiento de pesos ≈ 0,1 (no en sesgos ni normalizaciones), recorte de gradiente a
  1,0, calentamiento lineal (cientos a pocos miles de pasos) y decaimiento coseno hasta ~10 % del pico.
- Barre la tasa de aprendizaje en un modelo pequeño y extrapola con cuidado; el tamaño de lote efectivo
  (lote × acumulación × dispositivos) debe estar en tokens y quedar registrado.
- Usa bf16 para el cómputo y fp32 para los estados del optimizador y las acumulaciones.

## 5. Estabilidad y operación

- Registra por paso: pérdida, norma del gradiente, tasa de aprendizaje, tokens vistos y tokens/s. Una norma de
  gradiente que sube antes de un pico de pérdida avisa con margen.
- Ante un pico: no sigas a ciegas. Vuelve al último checkpoint bueno, salta los lotes culpables o baja la
  tasa de aprendizaje; documenta qué hiciste.
- Guarda checkpoints completos (pesos, optimizador, planificador, estado de RNG y posición en los datos) de
  forma atómica y conserva varios. Usa safetensors para los pesos que se compartirán.
- Define criterios de parada: presupuesto, pérdida de validación estancada, divergencia. Una corrida sin
  criterio de parada se alarga sola.

## 6. Entrega

Deja una ficha de la corrida (configuración, versión del código y de los datos, semillas, curvas, evaluaciones)
para poder repetirla o compararla. Evalúa con un conjunto retenido y con tareas reales antes de declarar éxito
(ver la skill de evaluación).
