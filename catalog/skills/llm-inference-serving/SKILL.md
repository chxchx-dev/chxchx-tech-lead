# Ejecutar y servir modelos de lenguaje

Servir es otro problema que entrenar: importan la memoria, la latencia, el costo por token y la seguridad del
servicio. Mide antes de optimizar y valida cada optimización.

## Presupuesto de memoria

- Pesos ≈ parámetros × bytes por peso (fp16/bf16: 2; int8: 1; 4 bits: ~0,5).
- **KV cache** por token ≈ 2 × capas × cabezas KV × dimensión de cabeza × bytes. Multiplícalo por el contexto
  máximo y por las peticiones simultáneas: suele dominar. GQA y cuantizar el KV lo reducen.
- Deja margen (10 a 20 %) para activaciones y fragmentación. Un fallo por falta de memoria en producción
  indica que no se hizo esta cuenta.

## Elegir motor

- **llama.cpp / GGUF / Ollama:** una máquina, CPU o GPU modesta, despliegue sencillo, buenas cuantizaciones.
- **vLLM, TGI, SGLang:** GPU y mucha concurrencia; lotes continuos, PagedAttention y prefijos compartidos.
- Para entrenar y evaluar, usa el mismo tokenizador y plantilla que en servicio; verifica el modelo
  convertido contra el original en un conjunto de prompts.

## Cuantización con validación

1. Mide el modelo original (perplejidad y tareas propias).
2. Cuantiza (GGUF Q4_K_M/Q5_K_M/Q8_0, AWQ, GPTQ, FP8) y repite las **mismas** mediciones.
3. Acepta solo si la caída está dentro del umbral acordado; las tareas de razonamiento, código y de otros
   idiomas se degradan primero, así que inclúyelas.
4. Registra el método, el conjunto de calibración y los hashes de los archivos resultantes.

## Rendimiento

- Mide por separado: tiempo hasta el primer token (prefill), tokens por segundo (decodificación) y
  rendimiento total bajo carga. Usa prompts con las longitudes reales.
- Ajusta tamaño de lote, contexto máximo, paralelismo y compilación solo con una medición antes y después.
- Haz una prueba de carga con concurrencia creciente y busca el punto donde la latencia se dispara; fija el
  límite de peticiones por debajo de él.

## Servicio

- Valida y limita la entrada: tamaño de prompt, `max_tokens`, número de peticiones por cliente y tiempo
  máximo. Cancela la generación si el cliente se desconecta.
- Transmite la respuesta (streaming) y define tiempos de espera; devuelve errores claros cuando la cola está
  llena en vez de acumular peticiones.
- Autentica el servicio, no lo expongas sin control a internet y trata el texto del usuario como no confiable
  (ver la skill de validación de seguridad).
- Observabilidad: latencias por etapa, tokens por petición, ocupación de la cola y de la memoria, tasa de
  errores, y una verificación de salud que ejecute una generación corta real.

## Determinismo y reproducibilidad

Con temperatura 0 el resultado puede variar según el lote y el hardware; si la reproducibilidad importa, fija
versión del motor, de los kernels y de los controladores y documenta la tolerancia.
