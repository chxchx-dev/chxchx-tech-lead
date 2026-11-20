# Observabilidad

Debes poder responder, sin desplegar código nuevo, **qué pasó, a quién afectó y por qué**.

## Registros

- Estructurados (JSON o pares clave-valor), con nivel, marca de tiempo, nombre del servicio, versión y un
  identificador de correlación que acompañe cada petición entre servicios.
- Registra eventos y decisiones con contexto, no solo "error". Un error se registra una vez, donde se maneja,
  con la causa.
- **Nunca** registres secretos, tokens, contraseñas ni datos personales sin redactar. Limita el tamaño de los
  campos.
- Niveles con criterio: `error` exige acción; `warn` anuncia un problema que se auto-recuperó; `info` cuenta el
  flujo normal con moderación.

## Métricas

- Por servicio, los cuatro señales de oro: **latencia** (percentiles, no solo media), **tráfico**, **errores**
  y **saturación**. Para recursos: utilización, saturación y errores (USE).
- Cuida la cardinalidad: no uses identificadores de usuario o de petición como etiquetas.
- Instrumenta con estándares abiertos (OpenTelemetry) para no atarte a un proveedor.

## Trazas

- Propaga el contexto de traza en cada salto (HTTP, colas, tareas en segundo plano). Muestrea con criterio y
  conserva siempre las trazas con error.

## Objetivos y alertas

- Define SLO desde la experiencia de la persona usuaria (disponibilidad y latencia de las operaciones que
  importan) y un presupuesto de error.
- Alerta sobre **síntomas** que requieren acción humana inmediata (quemado rápido del presupuesto), no sobre
  cada causa posible. Cada alerta enlaza a una guía de actuación (runbook). Revisa y elimina las que nadie
  atiende.

## Operación

- Paneles por servicio con las señales de oro y los despliegues marcados.
- Comprobaciones de salud que ejerciten dependencias críticas sin provocar cascadas.
- Tras un incidente: línea de tiempo, causa, qué señal faltó y el cambio de instrumentación que se agrega.
