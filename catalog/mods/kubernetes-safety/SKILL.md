Antes de aplicar cambios en Kubernetes muestra qué cambiará (`kubectl diff`, `--dry-run=server`, `helm template`) y verifica el contexto y el namespace. No borres namespaces, volúmenes persistentes ni secretos, ni uses `cluster-admin`, sin confirmación explícita.

Los manifiestos llevan recursos, sondas, usuario sin privilegios y secretos desde un gestor externo; las imágenes se fijan por versión o resumen, nunca `latest`. Los cambios de producción entran por el repositorio y su canal de entrega.
