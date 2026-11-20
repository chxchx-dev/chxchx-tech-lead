# Rendimiento en GPU y entrenamiento distribuido

La regla es la de siempre: **mide, localiza el cuello de botella, cambia una cosa, vuelve a medir**.

## Qué medir

- Tokens por segundo y **MFU** (FLOPs útiles frente al pico del hardware). Un modelo denso bien optimizado
  suele pasar del 40 %; por debajo del 20 % hay un problema que encontrar.
- Utilización de la GPU, memoria usada y pico, tiempo de datos frente a tiempo de cómputo y tiempo de
  comunicación entre dispositivos.
- Usa el perfilador (`torch.profiler`, Nsight) sobre unos pocos pasos ya calientes; descarta los primeros
  pasos (compilación, cachés).

## Cuellos de botella típicos

1. **Datos:** la GPU espera. Más trabajadores, preprocesado previo, formato binario por fragmentos, lectura
   asíncrona.
2. **Kernels pequeños y sincronizaciones:** fusiona operaciones (`torch.compile`), evita `.item()` y copias.
3. **Atención:** usa implementaciones eficientes (FlashAttention / SDPA); el costo cuadrático domina con
   contextos largos.
4. **Comunicación:** all-reduce lento por red o por tamaño de lote demasiado pequeño por dispositivo;
   solapa comunicación y cómputo, ajusta buckets y usa la topología correcta (NVLink dentro del nodo).
5. **Precisión:** bf16/FP8 donde el modelo lo tolere y se haya validado.

## Memoria

- Orden de medidas, de menor a mayor costo: bf16, lote menor con acumulación, checkpointing de activaciones
  (≈ +30 % de cómputo), optimizador en 8 bits, sharding (FSDP/ZeRO-2/3), descarga a CPU, paralelismo de
  tensores o de tubería.
- Una falta de memoria se triagea con la memoria pico por fase (modelo, optimizador, activaciones) y con
  `torch.cuda.memory_summary`, no probando valores al azar.
- Vigila la fragmentación (`expandable_segments`) y las fugas por tensores retenidos entre pasos.

## Entrenamiento distribuido

- Empieza por datos paralelos (DDP/FSDP); añade paralelismo de tensores o de tubería solo cuando el modelo o
  el contexto lo exijan.
- Comprueba equivalencia con un dispositivo (pérdida y gradientes dentro de tolerancia) antes de escalar.
- Que cada rango registre lo que hace y que un fallo en uno detenga a todos con un mensaje claro; define
  tiempos de espera de colectivas para no colgar un clúster entero.
- Checkpoints distribuidos que se puedan reanudar con otro número de dispositivos si es necesario.

## Validación de optimizaciones

Toda optimización debe demostrar que (a) acelera, con la medición antes/después, y (b) **no cambia el
resultado** más allá de la tolerancia acordada (misma curva de pérdida, mismas métricas).
