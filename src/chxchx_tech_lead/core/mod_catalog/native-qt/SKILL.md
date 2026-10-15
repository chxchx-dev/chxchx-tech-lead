# Aplicaciones nativas C++ y Qt multiplataforma

Mantén C++20/Qt 6 portable entre Windows, macOS y Linux. Separa UI, casos de uso e integraciones; usa Qt Widgets cuando el proyecto sea una aplicación de escritorio nativa y evita WebEngine si no es un requisito explícito.

Usa CMake con targets y dependencias explícitas, rutas por `QStandardPaths`, ejecución asíncrona para procesos y `QProcess` con argumentos separados. Aísla diferencias de plataforma y documenta requisitos mínimos. No asumas que compilar en una distribución hace portable el binario: despliega runtime/plugins, comprueba dependencias externas, genera paquete por SO y valida el artefacto en un runner de ese SO.

Para cambios de UI conserva controles nativos y accesibilidad. Considera rutas con espacios, Unicode, permisos, cierre de procesos, escalado HiDPI y las diferencias entre Intel/ARM. Informa qué plataformas se compilaron realmente y cuáles siguen sin validar.
