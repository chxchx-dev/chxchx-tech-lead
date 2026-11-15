Usa imágenes base con versión fijada, construcción multietapa y un usuario sin privilegios. Ordena las capas para aprovechar la caché (dependencias antes que el código), añade `.dockerignore` y no copies `.env`, llaves ni el historial de git a la imagen.

Pasa la configuración por variables de entorno o secretos del orquestador, define `healthcheck` y límites de recursos cuando aplique, y no publiques puertos que no hagan falta. Verifica la imagen construyéndola y, si es posible, escaneándola antes de dar el cambio por bueno.
