# Ingeniería de datos

## Diseño de pipelines

- **Idempotencia:** ejecutar una etapa dos veces con la misma entrada produce el mismo resultado sin duplicar.
  Escribe por partición con reemplazo atómico o con claves de fusión (`MERGE`), no con "añadir al final".
- Separa etapas con contratos claros: crudo (inmutable) → limpio → modelado → servicio. Nunca modifiques el crudo.
- Determinismo: la lógica depende de la entrada y de la fecha lógica de la ejecución, no de la hora del
  reloj. Eso permite reprocesar el pasado.
- Maneja lo tardío y los duplicados: define claves naturales y ventanas de tolerancia.

## Contratos y validación

- Esquema explícito por conjunto de datos (tipos, nulabilidad, claves, rangos, valores permitidos) y versionado;
  un cambio incompatible se anuncia y se migra.
- Validaciones en cada frontera (Great Expectations, dbt tests, pandera, comprobaciones propias):
  unicidad de claves, no nulos, integridad referencial, rangos, frescura y volumen esperado.
- Una validación fallida **detiene** la etapa y avisa; los datos dudosos van a una cuarentena para revisarlos,
  no se descartan en silencio.
- Mide la calidad con el tiempo (tasa de nulos, duplicados, retraso) y alerta sobre desviaciones.

## SQL y rendimiento

- Particiona por fecha o por una clave de filtrado habitual y usa formatos columnares (Parquet) con
  compresión; evita muchos archivos pequeños.
- Filtra pronto, selecciona solo las columnas necesarias, revisa los planes de ejecución y evita productos
  cartesianos accidentales en uniones.
- Prueba las transformaciones con conjuntos pequeños y casos límite (vacío, duplicados, nulos, zonas horarias).

## Operación

- Reintentos acotados con retroceso; las tareas deben poder reanudarse desde el último punto correcto.
- Linaje: de dónde viene cada columna y qué la consume, para evaluar el impacto de un cambio.
- Datos personales: clasifícalos, minimiza, controla el acceso y respeta los plazos de retención y de borrado.
- Secretos fuera del código y credenciales con el mínimo permiso.
