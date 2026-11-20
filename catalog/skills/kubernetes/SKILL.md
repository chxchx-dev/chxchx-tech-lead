# Kubernetes

## Cada carga de trabajo

- **Recursos:** `requests` y `limits` de CPU y memoria reales (medidos). Sin `requests` el planificador adivina;
  un límite de memoria bajo mata el contenedor.
- **Sondas:** `readinessProbe` (¿puede recibir tráfico?), `livenessProbe` (¿hay que reiniciarlo?) y
  `startupProbe` para arranques lentos. La de vida no debe depender de servicios externos.
- **Réplicas y disponibilidad:** al menos 2 réplicas para servicios de usuario, `PodDisruptionBudget`,
  `topologySpreadConstraints` y estrategia de despliegue con `maxUnavailable` acotado.
- **Cierre ordenado:** maneja `SIGTERM`, ajusta `terminationGracePeriodSeconds` y usa `preStop` si hay que
  drenar conexiones.

## Seguridad

- Ejecuta sin root (`runAsNonRoot`), sistema de archivos de solo lectura, `allowPrivilegeEscalation: false`,
  elimina capacidades (`drop: ["ALL"]`) y usa `seccompProfile: RuntimeDefault`.
- Cuentas de servicio con el mínimo RBAC; evita `cluster-admin`. `NetworkPolicy` con denegación por defecto.
- Imágenes por resumen (digest) o etiqueta inmutable, de registros de confianza y escaneadas; nunca `latest`.
- **Secretos:** no en el repositorio ni en variables en claro dentro de manifiestos. Usa un gestor externo
  (External Secrets, Sealed Secrets, SOPS) y limita quién puede leerlos.

## Configuración y entornos

- Un solo conjunto de manifiestos con variaciones por entorno (Kustomize o Helm con `values` por entorno);
  evita copiar y pegar.
- `ConfigMap` para configuración, con recarga controlada; versiona los cambios junto al código.

## Despliegue y operación

- Despliegue gradual (rolling, canario o azul/verde) con una condición de aborto y reversión probada.
- Observabilidad: métricas, registros y trazas por servicio; alertas sobre síntomas del usuario, no solo
  sobre uso de CPU.
- Antes de aplicar: `kubectl diff`, `kubectl apply --dry-run=server`, `helm template | kubeconform` y
  análisis de políticas (kube-linter, Kyverno/OPA). Revisa lo que cambiará, sobre todo borrados.
- No toques producción a mano: todo cambio entra por el repositorio y su canal de entrega.
