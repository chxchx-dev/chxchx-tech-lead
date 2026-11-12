# Roadmap de versiones y actualizaciones

- Fecha: 2026-11-12.
- Estado: propuesta de numeración y evolución; no hay releases nuevos publicados.
- Producto único: CLI, TUI, Studio, runtime, catálogos y contratos se entregan
  con una versión común. No hay calendarios ni releases independientes por UI.
- Ejecución técnica: [ROADMAP.md](ROADMAP.md). Arquitectura:
  [RUST-MIGRATION.md](RUST-MIGRATION.md).

## 1. Secuencia propuesta de versiones

Los números expresan objetivos y orden, no fechas ni disponibilidad. Antes de
reservar un tag se contrasta con la última versión realmente publicada; si la
línea 0.5 avanza, se ajusta el número siguiente sin cambiar las puertas de salida.

| Línea | Objetivo | Condición para entregarla |
| --- | --- | --- |
| `0.5.x` legado | Correcciones necesarias de seguridad, datos y uso diario durante la transición | Suite vigente y QA del cambio; evitar ampliar el legado salvo necesidad concreta. |
| `0.6.0-dev.N` privado | Iteración local/CI del port antes de alpha | No se publica en los canales actuales ni se anuncia como prerelease; no es una versión instalable para usuarios. |
| `0.6.0-alpha.N` | Producto unificado Rust en evaluación opt-in | R0–R2 completos para la primera alpha; siguientes alphas incorporan capacidades R3–R6 con cobertura y limitaciones declaradas. |
| `0.6.0-beta.N` | Paridad funcional y validación de migración | R3–R6 completos; pruebas de datos, permisos y uso simultáneo; sin funciones imprescindibles pendientes. |
| `0.6.0-rc.N` | Candidato a reemplazar la instalación actual | Matriz de plataforma/paquetes y R7 comprobados; solo correcciones de bloqueantes. |
| `0.6.0` | Primer producto Rust estable con todos los modos | Paridad, rendimiento acordado, instalación/actualización/rollback y QA real aprobados; retiro del legado en el checkout validado. |
| `0.6.x` | Estabilización posterior al corte | Correcciones compatibles y regresiones cubiertas; sin ampliar implícitamente el formato de datos. |
| `0.7.0` | Fluidez, accesibilidad y experiencia de uso | Mejoras elegidas por evidencia: navegación, atajos, estados, repos grandes y consumo; presupuestos y QA medidos. |
| `0.8.0` | Inteligencia de lenguaje y problemas | LSP administrado como proceso del workspace: diagnósticos, completar, definición/referencias y panel de problemas; cuotas/cancelación. |
| `0.9.0` | Revisión de cambios y colaboración con agentes | Stage por hunk, comparación de diffs y aprobación de propuestas visuales según soporte real del proveedor; backups y límites preservados. |
| `1.0.0` | Contratos y soporte estables del producto | Plataformas y política de soporte publicadas, contratos estabilizados, documentación completa y distribución/recuperación fiables. |

Las mejoras de 0.7–0.9 se priorizan después del corte usando necesidades reales;
no son condiciones nuevas de la migración ni compromisos cerrados de alcance.
Funciones Git como rebase o push forzado siguen excluidas por ADR 0011 salvo una
decisión posterior explícita; no se habilitan por aparecer una nueva versión.

## 2. Reglas de numeración y compatibilidad

- Una versión de producto, también para builds CLI sin GUI. La lista de features
  incluidas se publica por artefacto; un paquete reducido no es otra línea de producto.
- Durante el legado manda `pyproject.toml`. Al preparar R2 se usa un metadato
  Cargo explícito para la evaluación Rust, sin reemplazar la versión instalada.
  Al cortar R7, Cargo pasa a ser la única fuente y el pipeline valida que todos
  los artefactos, launchers y manifiestos reporten la misma versión de producto.
- Política del proyecto: patch corrige sin romper contratos públicos; minor
  incorpora capacidades. Antes de 1.0, cualquier ruptura exige notas, migración
  y opt-in explícitos. Desde 1.0, las rupturas públicas requieren major.
- Versiones del bridge, eventos, IPC, configuración y manifiesto son independientes
  del número del producto. No incrementar un schema solo por publicar una release.
- Mantener nombres de comandos/opciones y schemas existentes si son compatibles.
  Toda retirada tiene aviso, alternativa, escenario de migración y validación.
- Alpha/beta/RC indican pruebas del mismo producto; no tres ediciones separadas.
  No afirmar que toda alpha tiene paridad o es apta para datos únicos de producción.

## 3. Canales y publicaciones

Conservar `stable` y `dev` de ADR 0013. `stable` solo recibe versiones aprobadas;
  `dev` es opt-in y nunca sustituye silenciosamente una instalación estable.
Las versiones locales `0.6.0-dev.N` viven solo en esta rama y CI. Los prereleases
Rust alpha se ofrecen explícitamente en el flujo de evaluación de dev,
en rutas propias, con versión inmutable y features declaradas. No crear un tercer
canal sin justificar un cambio de manifiesto y compatibilidad de instaladores.

El usuario puede instalar la variante completa o solo consola. Ambas conservan
versión, políticas y formatos; actualizar respeta las features instaladas y no
descarga Studio o sus dependencias por una actualización CLI ordinaria.
Si una capacidad no está compilada, el comando informa cómo obtenerla sin
instalarla por su cuenta ni fallar por bibliotecas gráficas ausentes.

Publicar requiere evidencia por target: versión/commit, SO y arquitectura,
capacidades, schemas/compatibilidad mínima, checksums, avisos de licencia,
changelog, notas de migración y estado de firma. La evolución de estos metadatos
debe versionar el manifiesto si rompe a clientes existentes. Un checksum valida
integridad frente al manifiesto; autenticidad requiere firma y una raíz de
confianza. Firma/notarización y política de verificación quedan como tareas de
R7, sin afirmar que hoy estén operativas ni guardar claves en el repositorio.

## 4. Flujo común de actualización

La capacidad `distribution` concentra el flujo para CLI, TUI y Studio. Las
interfaces presentan el mismo plan, progreso, errores y resultado.

1. Consultar el canal elegido por el usuario; mostrar versión, cambios,
   compatibilidad, descarga y efectos. Ofrecer dry-run sin aplicar cambios.
2. Descargar a staging user-local y validar origen/firmas según la política
   implementada, integridad, target y compatibilidad antes de ejecutar el paquete.
3. Detectar procesos y sesiones activos. Actualizar en un punto seguro o pedir
   cierre explícito; nunca reiniciar agentes o transferir aprobaciones por actualizar.
4. Respaldar datos que requieran migración y probar precondiciones. El cambio de
   versión no amplía trust, permisos ni alcance de archivos/red.
5. Instalar en carpeta versionada, validar versión/arranque y cambiar solo
   launchers/archivos administrados por ChxChx cuando el paquete esté listo.
6. Conservar versión anterior y recuperación documentada. Ante un fallo,
   restaurar launcher y, si corresponde, la copia de datos previa a la migración.
7. Registrar resultado sin secretos. No modificar proyectos, configuración de
   terceros o PATH global fuera de los efectos previamente mostrados.

Rollback de binarios no basta si cambió el formato de datos. Cada migración debe
declarar si es reversible, qué backup necesita y qué versión puede abrirlo.
Los clientes y supervisores activos negocian compatibilidad; no convivirán
versiones incompatibles sobre el mismo estado. El upgrade no mata procesos
automáticamente para forzar un cambio de runtime.

## 5. Checklist de una release

- [ ] Alcance cerrado y changelog por capacidades, con efectos por modo.
- [ ] Paridad y fronteras verificadas; formato, lints, unitarias, contratos y E2E.
- [ ] Build completo y build CLI sin GUI, con features publicadas por artefacto.
- [ ] Versiones de producto, protocolos y datos contrastadas con compatibilidad.
- [ ] Matriz Windows/macOS/Ubuntu/Fedora con evidencia; QA interactivo aplicable.
- [ ] Medidas de rendimiento comparadas con presupuestos y release anterior.
- [ ] Instalar, abrir, actualizar, fallar durante actualización, rollback y
  desinstalar en máquinas limpias, preservando datos y rutas no administradas.
- [ ] Licencias, checksums y política de firma/verificación comprobados.
- [ ] Notas de migración, limitaciones y soporte publicables con el artefacto.
- [ ] Publicación realizada solo como acción explícita posterior a la validación.

## 6. Mantenimiento y actualizaciones futuras

- Durante la transición se corrigen fallos críticos del legado; no congelarlo
  mientras los usuarios dependan de él. Fin de soporte tras Rust estable y un
  camino de migración/recuperación validado, con aviso previo.
- Reservar patches para regresiones, seguridad, integridad de datos y roturas
  de protocolos de proveedores. No esperar una minor para corregir un riesgo activo.
- Revisar dependencias, licencias, MSRV y mínimos de SO al planear cada minor;
  cualquier subida se documenta y se comprueba en CI antes de publicarse.
- Protocolos de agentes se prueban contra versiones identificadas y fixtures;
  un cambio de proveedor se resuelve en `integrations/`, sin alterar cada UI.
- Tras 1.0, mantener la minor estable anterior durante la transición a su sucesora
  cuando haya correcciones críticas; concretar plazos y recursos antes de prometer LTS.
- Sin actualización automática por defecto: avisar y solicitar aplicar el plan.
  Una eventual automatización requiere opt-in y las mismas garantías de backup,
  sesiones activas, compatibilidad y rollback.
