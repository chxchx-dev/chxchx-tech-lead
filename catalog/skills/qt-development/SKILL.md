# Aplicaciones nativas con Qt

- Usa la versión mínima de Qt y los módulos que el producto ya soporta; declara cada módulo requerido en el build y evita introducir dependencias innecesarias.
- Sigue el modelo de interfaz existente (Widgets o QML). No mezcles ambos para una pantalla sin una necesidad clara.
- Define una sola autoridad para la propiedad de cada `QObject`: parent-child de Qt o smart pointer, sin doble propiedad. Mantén claros el hilo de afinidad y el orden de cierre.
- Usa signals/slots para comunicar cambios entre objetos. Mantén la lógica de dominio fuera de callbacks de widgets y componentes de presentación.
- Toda mutación de UI ocurre en el hilo GUI. Ejecuta E/S o trabajo costoso de forma asíncrona con cancelación, señales encoladas y cierre seguro.
- Para listas grandes, usa model/view en vez de crear un widget por fila. Conserva estados vacíos, carga, error, selección y accesibilidad del teclado.
- Respeta la escala HiDPI, el foco, la navegación por teclado, el tema del sistema y la traducción que ya use el producto.
- Evita Qt WebEngine en software nativo salvo que el requerimiento lo necesite expresamente; prefiere controles Qt y APIs del sistema.
