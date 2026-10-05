# Product UI/UX design

Diseña para que las personas completen sus tareas con claridad. Antes de
implementar, identifica el usuario, su objetivo principal, la información que
necesita y la acción más importante de la pantalla.

## Estructura y jerarquía

- Da a cada pantalla un propósito reconocible y un título que describa la tarea.
- Ordena el contenido por importancia y frecuencia de uso. Mantén juntas las
  acciones y los datos relacionados; reserva la atención visual para la acción
  principal.
- Divide formularios y flujos largos en pasos entendibles. Pide solo los datos
  necesarios en cada momento y explica errores junto al campo que deben corregir.
- Usa patrones familiares y consistentes con el producto. Introduce un patrón
  nuevo solo cuando resuelva una necesidad concreta.

## Sistema visual

- Revisa primero los componentes, tokens, tipografía, colores y espaciado que ya
  usa el producto. Amplíalos de forma coherente antes de crear estilos paralelos.
- Define una jerarquía tipográfica corta y una escala de espaciado consistente.
  Mantén contraste suficiente y no uses solo el color para comunicar estado.
- Usa alineación, espacio y agrupación para explicar relaciones. Evita
  decoración que compita con el contenido o no ayude a la tarea.
- Conserva una apariencia propia del producto. Evita pantallas genéricas con
  gradientes, tarjetas redundantes, títulos repetidos o adornos sin función.

## Interacción y adaptación

- Diseña estados de carga, vacío, error, éxito, deshabilitado y foco, además del
  estado ideal. Conserva la entrada del usuario cuando un envío falla.
- Usa controles semánticos con nombres accesibles, operación por teclado, foco
  visible y mensajes de validación comprensibles.
- Adapta jerarquía y navegación a móvil, tableta y escritorio. Evita depender de
  hover, apuntar a controles táctiles cómodos y mantener el contenido legible con
  zoom y texto ampliado.
- Respeta las preferencias de movimiento reducido y evita animaciones que
  retrasen una acción frecuente.

## Verificación

- Comprueba la pantalla en anchos estrechos y amplios, y revisa todos los
  estados de interacción, no solo la captura inicial.
- Valida contraste, navegación por teclado, etiquetas de controles y errores de
  formularios. Usa las herramientas de accesibilidad y pruebas que ya tenga el
  proyecto.
- Mantén los cambios acotados al flujo solicitado y explica cualquier decisión
  visual que afecte navegación, contenido o comportamiento.
