Trata Redis como caché o cola, no como única fuente de verdad. Pon siempre expiración (TTL) a las claves de caché, usa prefijos por servicio y entorno, y evita claves o valores sin límite de tamaño. Diseña para que la aplicación siga funcionando, degradada, si Redis no responde.

Evita `KEYS` en producción (usa `SCAN`), agrupa operaciones con pipelines, protege contra avalanchas de caché con TTL escalonados y no guardes secretos ni datos personales sin cifrar. Documenta qué claves existen, su formato y su duración.
