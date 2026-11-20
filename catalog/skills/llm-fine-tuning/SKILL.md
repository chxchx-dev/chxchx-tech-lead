# Ajuste fino de modelos

Ajustar no es entrenar de cero: se parte de un modelo capaz y se corre el riesgo de empeorarlo en lo que no
se mide. Todo cambio se valida contra el modelo base.

## Antes de empezar

- Define la tarea y cómo se mide el éxito. Si no puedes escribir 50 casos de evaluación, aún no sabes qué
  quieres.
- Comprueba la **licencia** del modelo base y de los datos (¿se permite el uso derivado y comercial?).
- Evalúa el modelo base con tu conjunto de evaluación y con unas pruebas generales (capacidad que no quieres
  perder). Sin esa línea base no puedes demostrar que mejoraste.
- Prueba primero con ingeniería de prompts o recuperación: es más barato y a veces suficiente.

## Datos

- Pocos datos, muy buenos y con el formato exacto de uso, rinden más que muchos ruidosos. Revisa una muestra
  a mano.
- Usa la **misma plantilla de chat** que el modelo base. Una plantilla distinta es una de las causas más
  comunes de resultados malos.
- Entrena solo sobre la respuesta: enmascara las instrucciones y el sistema en la pérdida (etiquetas −100).
- Separa evaluación por documento o por conversación; elimina duplicados entre entrenamiento y evaluación.

## Método

- **LoRA/QLoRA** para la mayoría de los casos: rangos de 8 a 64, aplicado a todas las proyecciones lineales;
  la tasa de aprendizaje suele ser mayor que en ajuste completo (1e-4 a 2e-4) y menor en ajuste completo
  (1e-5 o menos).
- Pocas épocas (1 a 3). Detén el entrenamiento cuando la evaluación se estanca; más épocas memorizan.
- **Preferencias (DPO y similares):** parte de un modelo ya ajustado, con pares limpios y consistentes; vigila
  que no crezca la longitud de las respuestas ni la verbosidad sin que mejore la calidad.

## Evaluación obligatoria antes de adoptarlo

1. La métrica de la tarea frente al modelo base, con varias semillas y un intervalo de confianza.
2. **Olvido catastrófico:** las pruebas generales no deben caer más de un umbral acordado.
3. Seguridad y rechazos: que el ajuste no haya quitado salvaguardas que debían conservarse.
4. Casos adversos y de formato (JSON válido, longitud, idioma).

## Entrega

- Guarda el adaptador, la configuración, la versión de los datos y las métricas. Si fusionas el adaptador con el
  modelo, verifica que el modelo fusionado reproduce los resultados del adaptador y cuantiza después, no antes.
- Documenta limitaciones y el alcance para el que se validó.
