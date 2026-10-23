# ADR-0007: CD optativo a VPS para proyectos Docker Compose

- Estado: primera implementación en CLI/TUI/Studio
- Fecha: 2026-10-15
- Alcance: despliegue de proyectos de usuario; no el pipeline de ChxChx

## Contexto

El usuario quiere que ChxChx facilite llevar sus proyectos a un servidor propio
o VPS, con automatización opcional. El formato de runtime no estaba fijado. La
primera entrega necesita un contrato acotado, credenciales fuera del checkout,
previsualización, confirmación y recuperación ante errores.

## Decisión

- La primera versión genera, bajo petición explícita, un workflow GitHub Actions
  para proyectos con exactamente un manifiesto Docker Compose reconocido en la
  raíz. No altera el workflow CI ni automatiza despliegues del repositorio
  ChxChx.
- El despliegue es manual con `workflow_dispatch` y una confirmación booleana
  requerida. El usuario puede habilitar `push` en una rama específica mediante
  `--auto-deploy-on`; el trigger no se activa por defecto.
- GitHub Actions usa SSH al VPS, una GitHub Environment llamada `production`,
  permisos `contents: read`, llave privada y `known_hosts` en secrets, además
  de host/usuario/ruta y puerto opcional en variables. La identidad del host se
  valida estrictamente; ChxChx no aprende claves de host durante la conexión ni
  guarda secretos localmente.
- El VPS debe tener un clon existente del repositorio, `origin` coincidente,
  árbol versionado limpio y Docker Compose. El workflow obtiene el commit
  seleccionado, construye e inicia los servicios y espera que los servicios
  persistentes estén running y sus healthchecks no indiquen error.
- Ante un fallo se intenta volver al commit anterior y reconstruirlo. Esta
  reversión no puede deshacer cambios en bases de datos, volúmenes externos ni
  otros efectos de la aplicación; las migraciones deben ser compatibles con la
  versión previa.
- CLI, TUI y Studio llaman los mismos casos de uso. `bridge deploy` expone solo
  estado JSON versionado. El setup y la retirada usan el gestor común de
  workflows para conservar archivos ajenos, detectar modificaciones y crear
  backups fuera del checkout.

## Límites iniciales

- Solo un manifiesto Compose base en la raíz; los overrides que Docker Compose
  carga automáticamente siguen bajo el control del proyecto.
- Se esperan servicios de larga duración. Proyectos con tareas de una sola
  ejecución, varias apps, estrategias blue/green, migraciones automáticas o
  despliegue sin downtime necesitan una plantilla especializada posterior.
- El usuario configura el VPS, el clon, Docker, GitHub Environment, secrets,
  variables y política de aprobación. En repositorios privados, el VPS requiere
  su propia llave de lectura para obtener commits.
- Un usuario no root reduce exposición del login, pero pertenecer al grupo
  `docker` suele conceder privilegios equivalentes a root; documentar Docker
  rootless como alternativa.

## Validación

La primera versión debe cubrir generación manual/automática, colisiones y
integridad/backup, confirmación del dispatch, estado bridge, logs, rollback
generado y presentación equivalente en CLI, TUI y Studio. El despliegue real
requiere un VPS de prueba configurado por el usuario y no se debe ejecutar como
parte de CI.
