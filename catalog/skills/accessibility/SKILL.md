# Accesibilidad

Una interfaz accesible funciona con teclado, lector de pantalla, zoom y distintas capacidades visuales y motoras.
Se construye desde el principio; arreglarla después cuesta más.

## Estructura y semántica

- Usa el elemento o control nativo correcto (botón, enlace, campo, lista, tabla) antes de recurrir a ARIA. Un
  `div` con `onClick` no es un botón.
- Jerarquía de encabezados coherente, regiones (`main`, `nav`) y un título de página único y descriptivo.
- Cada control tiene un **nombre accesible** (etiqueta visible asociada, no solo placeholder) y un rol y estado
  correctos; los iconos sin texto llevan etiqueta.

## Teclado y foco

- Todo lo que se hace con el ratón se puede hacer con teclado, en un orden de tabulación lógico.
- El foco siempre es visible y se gestiona al abrir y cerrar diálogos (atrapar dentro del diálogo y devolver al
  disparador). Sin trampas de teclado.
- Atajos que no choquen con los del sistema, lector de pantalla o navegador, y que se puedan cambiar.

## Visual

- Contraste mínimo: 4,5:1 para texto normal y 3:1 para texto grande y componentes de interfaz. No transmitas
  información solo con color.
- Admite zoom y tamaño de texto mayor sin perder contenido ni funciones; evita tamaños fijos que rompan el
  diseño. Objetivos táctiles de al menos 44×44 px.
- Respeta `prefers-reduced-motion` y evita animaciones o parpadeos que no se puedan pausar.

## Contenido dinámico y formularios

- Anuncia cambios importantes con regiones en vivo (`aria-live`) sin saturar. Los errores de validación se
  asocian al campo, explican cómo corregirlos y no dependen del color.
- Tiempos límite: avisa y permite extenderlos.

## Verificación

1. Herramientas automáticas (axe, Lighthouse, eslint-plugin-jsx-a11y): detectan solo una parte.
2. Recorrido completo **solo con teclado**.
3. Prueba con un lector de pantalla (NVDA, VoiceOver, Orca) en los flujos principales.
4. Zoom al 200 % y modo de alto contraste.
5. Cada defecto encontrado se convierte en una prueba de regresión.
