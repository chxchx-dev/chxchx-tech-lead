# Validación de uso real hacia `v1.0`

Esta checklist sirve para reunir evidencia de Terminal Workspace en proyectos y
sistemas reales. No presupone que una prueba pasó: completa el registro después
de cada sesión y conserva los fallos junto con el contexto necesario para
reproducirlos.

## Qué registrar en una sesión

Antes de iniciar, anota fecha, sistema operativo y versión, versión/commit de
ChxChx, proyecto (puede ser un alias si el nombre es privado) y herramientas
externas utilizadas. Registra también si Zellij estuvo disponible o se usó el
fallback de subprocess.

```bash
chxchx-tech doctor .
chxchx-tech workspace status .
chxchx-tech process list .
chxchx-tech resources .
```

Repite `resources` durante el trabajo y al finalizar. Es una captura puntual de
RAM, swap, CPU y procesos administrados, no una medición histórica. Para estimar
el costo propio del entorno, compara el equipo antes de iniciar el workspace con
el mismo conjunto de herramientas abierto; anota el método y no atribuyas a
ChxChx el consumo de procesos externos.

## Escenarios de aceptación

Ejercita únicamente proyectos confiables y procesos descartables cuando el
escenario pueda interrumpir trabajo.

- [ ] Iniciar un workspace con procesos `auto_start` y confirmar estado/PID.
- [ ] Adjuntarse a la sesión, salir de la TUI y comprobar que el workspace sigue
  activo.
- [ ] Suspender y reanudar; confirmar que los procesos configurados quedan en el
  estado esperado y que la sesión existente se reutiliza.
- [ ] Cambiar a otro proyecto y volver; confirmar que no quedan procesos del
  proyecto suspendido y que el proyecto anterior se recupera correctamente.
- [ ] Detener un proceso con hijos en un proyecto de prueba y confirmar que no
  quedan descendientes. No uses procesos de trabajo o datos importantes para
  esta prueba.
- [ ] En una sesión controlada, cerrar inesperadamente la terminal o la TUI y
  comprobar el estado recuperable. Anota qué quedó vivo antes de reiniciar o
  limpiar procesos.
- [ ] Si usas Zellij, repetir adjuntar/desadjuntar; si usas fallback, anotar las
  limitaciones observadas de persistencia.
- [ ] Al menos una vez por semana de uso, revisar `doctor`, `workspace status`,
  `process list` y `resources` y registrar estabilidad, incidentes y cambios.

No marques un escenario como aprobado si no se ejecutó en ese sistema operativo.
Un escenario bloqueado por falta de herramienta se registra como pendiente con
la herramienta ausente.

## Registro por sesión

Copia este bloque en cada entrada. No incluyas rutas privadas, secretos ni
contenido de logs con credenciales.

```text
Fecha/hora y zona:
SO / versión:
ChxChx (versión o commit):
Proyecto o alias:
Zellij / fallback:
Procesos y agentes usados:
Escenarios ejecutados:
RAM/swap/CPU al inicio:
RAM/swap/CPU durante uso:
RAM/swap/CPU al final:
Incidentes (pasos, resultado esperado/real, recuperación):
Resultado: aprobado / fallido / pendiente
Siguiente acción:
```

## Cobertura por plataforma antes de `v1.0`

| Plataforma | CI | Uso real | Escenarios manuales pendientes |
|---|---|---|---|
| Linux | Matriz aprobada (Python 3.11–3.13) | Registrar sesiones | Cierres inesperados y uso sostenido |
| macOS | Matriz aprobada (Python 3.11–3.13) | Registrar/revalidar sesiones | Shells, sesiones y recuperación |
| Windows 11 Pro nativo | Matriz aprobada (Python 3.11–3.13) | Registrar sesiones | Árbol de procesos y recuperación |
| WSL | Matriz Windows aprobada; no equivale a prueba WSL | Fuera de evidencia por ahora | Confirmar si permanece en alcance y probar si aplica |

Actualiza esta tabla solo con evidencia ejecutada. El CI valida la matriz
automatizada, pero no reemplaza las pruebas manuales ni el uso sostenido.
