# Refactorización segura

Antes de refactorizar, identifica el comportamiento público, los llamadores, las pruebas y las restricciones arquitectónicas afectadas. Busca reutilizar la estructura existente y limita el cambio a una responsabilidad.

Preserva contratos y compatibilidad salvo que el usuario solicite cambiarlos. Separa una reestructuración amplia en pasos pequeños; no mezcles limpieza ajena a la tarea.

Después del cambio, describe qué comportamiento se preservó y qué verificaciones se ejecutaron. Si no puedes validar un límite importante, indícalo con precisión.
