# Ingeniería C++ moderno

- Conserva el estándar y las convenciones que ya usa el proyecto; adopta C++20 cuando el producto y sus compiladores destino lo permitan.
- Modela propiedad y duración de vida de forma explícita con RAII, valores y tipos estándar. Usa punteros crudos solo como referencias no propietarias cuando la API lo requiera.
- Evita estado global mutable, macros para lógica y dependencias de plataforma en headers públicos. Mantén interfaces pequeñas y encabezados autocontenidos.
- Define una política coherente de errores y excepciones según las APIs existentes. No ocultes fallos ni conviertas errores en estados ambiguos.
- En concurrencia, documenta quién posee cada dato, cómo se sincroniza y cómo se cancela o cierra el trabajo antes de destruir sus recursos.
- Prioriza legibilidad y seguridad de tipos; mide antes de optimizar. Aísla código específico de compilador o sistema operativo detrás de una interfaz pequeña.
- Usa warnings y análisis disponibles en cada toolchain; añade comprobaciones focalizadas para cambios de comportamiento y errores de memoria o concurrencia.
