# Ingeniería con PyTorch

## Estructura

- Separa modelo (`nn.Module` puro, sin E/S), datos, bucle de entrenamiento y configuración. El modelo debe poder
  instanciarse y ejecutarse en una prueba sin GPU ni datos reales.
- Una configuración tipada y serializable (dataclass o pydantic) gobierna todo; no leas variables globales
  dentro del modelo.
- Mueve tensores a dispositivo y dtype de forma explícita en un solo lugar. No mezcles CPU y GPU sin
  querer: falla pronto con comprobaciones de dispositivo en las pruebas.

## Precisión y rendimiento

- bf16 con `torch.autocast` para el cómputo; mantén en fp32 las pérdidas, normalizaciones sensibles y los
  estados del optimizador. Con fp16 usa escalado de gradiente.
- `torch.compile` tras comprobar que el modelo es correcto; mide antes y después y vigila los reinicios de
  compilación por formas dinámicas.
- Evita sincronizaciones ocultas (`.item()`, `.cpu()`, impresiones) dentro del bucle caliente.
- `DataLoader`: varios `num_workers`, `pin_memory`, `persistent_workers` y prefetch; confirma que la GPU no
  espera por los datos (perfil o utilización).

## Determinismo

- Fija semillas de Python, NumPy y PyTorch, y el estado de los generadores por trabajador del `DataLoader`.
- `torch.use_deterministic_algorithms(True)` cuando necesites reproducibilidad exacta (acepta un costo);
  documenta lo que aun así varía (operaciones atómicas, versiones de CUDA).

## Checkpoints

- Guarda `state_dict` de modelo, optimizador, planificador, escalador y RNG, junto con el paso y la posición en
  los datos. Escribe a un archivo temporal y renómbralo (atómico).
- Para compartir pesos usa **safetensors**, no `pickle`: cargar un `.pt` desconocido ejecuta código. Con
  `torch.load` usa `weights_only=True`.
- Comprueba al cargar: claves esperadas, formas y dtype. No uses `strict=False` sin registrar lo que se ignora.

## Pruebas de un modelo

- **Formas:** la salida tiene la forma esperada para tamaños de lote y longitudes variados (incluido 1).
- **Flujo de gradientes:** tras un paso, todos los parámetros entrenables tienen gradiente finito y no nulo.
- **Sobreajuste de un lote:** la pérdida baja de forma clara.
- **Invariancias:** relleno que no altera resultados, causalidad en un decodificador, equivalencia entre
  lote y ejemplos sueltos, igualdad entre la ruta de entrenamiento y la de inferencia.
- **Valores no finitos:** detecta NaN/Inf con comprobaciones activables (`torch.autograd.set_detect_anomaly`
  en depuración, no en producción).

## Errores frecuentes

Olvidar `model.eval()` o `torch.no_grad()` al evaluar; acumular el grafo guardando pérdidas sin `.detach()`;
barajar con la misma semilla en cada época; dividir por una longitud de relleno; usar el mismo tensor de
máscara con otro tipo de dato.
