Organiza por módulos de dominio con controladores delgados y lógica en servicios; inyecta dependencias en lugar de importarlas directamente. Valida y transforma la entrada con DTOs y `ValidationPipe` (whitelist activada), centraliza errores con filtros de excepción y no devuelvas entidades internas ni datos sensibles.

Protege rutas con guards, mantén la configuración en `ConfigModule` sin secretos en el repositorio, versiona los cambios incompatibles de la API y documenta contratos (OpenAPI). Prueba servicios con unitarias y flujos críticos con e2e sobre una base de datos de prueba.
