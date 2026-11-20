# CI/CD

## Integración continua

- Cada cambio ejecuta, de forma **determinista**: formato, análisis estático, pruebas y compilación. Lo que pasa en
  local debe pasar igual en CI (mismas versiones fijadas, mismo comando documentado).
- Rápido: cachés de dependencias y de compilación con claves basadas en los archivos de bloqueo, paralelismo
  y trabajos separados. Un pipeline lento se salta; un pipeline que falla al azar se ignora: arregla o
  aísla las pruebas inestables, no las reintentes a ciegas.
- Matriz de sistemas operativos y versiones solo donde aporta información; un fallo en una plataforma se
  reproduce con sus propios registros (que el paso imprima el final de la salida si el fallo es difícil de ver).
- La rama principal siempre construye; protégela con revisión y comprobaciones obligatorias.

## Seguridad del pipeline

- Permisos mínimos para el token del trabajo; fija las acciones de terceros por commit (SHA) y revisa sus
  actualizaciones.
- Secretos solo donde se necesitan, nunca impresos ni disponibles para ramas de bifurcaciones (`pull_request`
  de desconocidos). Prefiere identidades federadas (OIDC) a claves de larga vida.
- Escaneo de dependencias, de secretos y de la imagen o artefacto final; firma y genera una lista de
  materiales (SBOM) cuando se publica.

## Entrega continua

- Construye **una vez** y promueve el mismo artefacto entre entornos; no recompiles por entorno.
- Versionado semántico y registro de cambios; etiqueta el commit exacto publicado y guarda sumas de
  verificación junto a los archivos.
- Despliegue gradual con comprobaciones de salud automáticas y reversión rápida y probada. Las migraciones de
  base de datos son compatibles hacia atrás para poder revertir el código.
- Primero un ensayo: ejecuta el flujo de publicación en modo "sin publicar" y revisa los artefactos antes de
  hacerlo real.

## Mantenimiento

- Los flujos son código: se revisan, se prueban con cambios pequeños y se documentan (cómo ejecutarlos y
  cómo depurarlos).
- Mide la duración y la tasa de fallos del pipeline y trátalas como métricas de salud del equipo.
