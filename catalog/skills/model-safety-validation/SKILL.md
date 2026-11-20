# Validación de seguridad de modelos y aplicaciones con LLM

Trata al modelo como un componente que puede equivocarse o ser manipulado. La seguridad se demuestra con
pruebas, no con intenciones.

## Amenazas que hay que probar

- **Inyección de instrucciones:** texto externo (páginas, documentos, correos, resultados de herramientas)
  que intenta cambiar el comportamiento. Prueba inyecciones directas e indirectas, en varios idiomas y
  codificaciones.
- **Fuga de información:** revelar el prompt del sistema, datos de otros usuarios, claves o datos de
  entrenamiento. Prueba extracción de memoria y de contexto.
- **Contenido dañino o no permitido:** categorías definidas por tu política; mide la tasa de rechazo correcto y
  de rechazo excesivo (negar peticiones legítimas).
- **Abuso de herramientas:** que el modelo llame a una herramienta con argumentos peligrosos o fuera de
  alcance (borrar, enviar, gastar).
- **Denegación de servicio y costo:** entradas enormes, bucles de herramientas, generaciones sin tope.

## Controles que no dependen del modelo

- Separa instrucciones y datos; marca el contenido externo como no confiable.
- Principio de mínimo privilegio para cada herramienta: permisos acotados, lista de acciones permitidas,
  confirmación humana antes de acciones con efectos o irreversibles.
- Valida **toda salida** antes de usarla: esquema (JSON Schema/pydantic), rangos, listas permitidas. Nunca
  ejecutes ni interpoles en SQL, shell o HTML una salida del modelo sin escapar.
- Límites de tamaño, de pasos, de tiempo y de gasto por petición y por usuario.
- Registra entradas, salidas y llamadas a herramientas con datos personales redactados, para auditar.

## Batería de pruebas

1. Un conjunto versionado de casos adversos por categoría, con el resultado esperado.
2. Se ejecuta en cada cambio de modelo, de prompt o de herramientas, y bloquea la publicación si baja de un
   umbral.
3. Pruebas de regresión por cada incidente real: cada fallo encontrado se convierte en un caso permanente.
4. Revisión humana periódica de una muestra de conversaciones reales (con consentimiento y privacidad).

## Antes de publicar un modelo o una función

- Evaluación de calidad y de seguridad completas con resultados guardados.
- **Tarjeta del modelo:** uso previsto y no previsto, datos y licencias, limitaciones, evaluaciones y sesgos
  conocidos.
- Licencias del modelo base y de los datos verificadas; hashes de los archivos distribuidos.
- Plan de monitoreo, de respuesta a incidentes y de reversión a la versión anterior.
- Sin secretos ni datos personales en los pesos, el repositorio ni los registros.
