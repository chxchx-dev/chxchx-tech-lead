Antes de escribir pruebas identifica el comportamiento que no debe romperse. Prefiere pruebas pequeñas y deterministas del comportamiento observable sobre pruebas de implementación; cubre el caso feliz, los bordes y los errores esperados, y añade una prueba que falle antes de corregir un defecto.

Aísla tiempo, red y sistema de archivos con dobles o directorios temporales. Si una prueba es intermitente, encuentra la causa en lugar de reintentarla. Termina diciendo qué ejecutaste, qué pasó y qué quedó sin cubrir.
