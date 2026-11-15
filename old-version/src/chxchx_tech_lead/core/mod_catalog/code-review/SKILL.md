# Revisión antes de integrar

Al revisar un diff, busca defectos concretos que puedan afectar a usuarios: regresiones, errores de lógica, compatibilidad, pérdida de datos, riesgos de seguridad y casos sin cubrir. Recorre el flujo afectado y consulta las llamadas relevantes; no te limites a comentar estilo.

Ordena los hallazgos por severidad e incluye archivo/línea, escenario que los dispara y efecto observable. Verifica primero con evidencia disponible. Distingue claramente un defecto confirmado de una duda. Si no encuentras defectos, dilo y menciona qué no pudiste comprobar. No edites archivos durante una revisión solo de lectura.
