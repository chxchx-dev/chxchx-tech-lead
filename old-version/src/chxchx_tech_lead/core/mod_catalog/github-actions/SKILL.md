Define `permissions` mínimos por trabajo, fija las acciones de terceros por versión o commit y no imprimas secretos. Usa caché de dependencias, `concurrency` para cancelar ejecuciones obsoletas y matrices solo donde aporten cobertura real.

Para fallos de CI lee primero el log del paso que falló, reproduce localmente con el mismo comando y corrige la causa, no el síntoma. Evita `pull_request_target` con código no confiable y separa compilar de publicar para que el publicar requiera aprobación o etiquetas.
