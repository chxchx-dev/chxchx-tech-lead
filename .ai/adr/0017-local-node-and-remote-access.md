# ADR 0017: Nodo local, acceso remoto y cliente PWA en Dioxus

- Estado: Aceptado por el mantenedor el 2026-11-20 (dirección y límites). Los detalles de cada fase
  se validan con su propio criterio de salida en [ROADMAP-HIBRIDO.md](../ROADMAP-HIBRIDO.md).
- Fecha: 2026-11-20.
- Relacionados: [ADR 0014](0014-chat-access-levels-and-permission-cards.md) (niveles de acceso),
  [ADR 0016](0016-rust-control-plane.md) (supervisor local bajo demanda, R4.1).

## Contexto

El mantenedor quiere seguir trabajando fuera de casa desde el celular o el navegador: abrir un
proyecto de la PC, hablar con su agente, aprobar permisos y recuperar resultados aunque se corte
la conexión. También quiere que el producto instale, vigile y repare sus propios componentes.

Situación del código (auditoría del 2026-11-20):

- Un solo crate `chxchx-tech` con las features `cli`, `tui` y `studio` (Iced 0.14, sin WebView).
  `src/features/` y `src/integrations/` no dependen de Iced, y la CI compila el build `cli` sin GUI.
- Los trabajos largos (turnos de agente, instalación de modelos locales) corren en hilos de la
  ventana (`studio_jobs.rs`) y mueren al cerrar Studio.
- El estado se guarda en JSON/JSONL por proyecto, con escritura atómica y redacción de secretos.
- El único servicio Python, `services/agent-adapter`, se lanza por turno y tiene un contrato
  versionado.
- El release se verifica con SHA-256, sin firma, y el actualizador solo abre la página de descarga.

## Decisión

1. **Un nodo local**, `chxchx-tech node`, detrás de la feature `node` (`tokio` y `axum` con
   versión exacta). Es el supervisor bajo demanda que ADR 0016 dejó como propuesta: dueño único de
   los trabajos de cada proyecto mientras corre, con un candado por proyecto. Sin nodo, Studio
   funciona como hoy.
2. **Los trabajos son explícitos**: `JobSpec`, un `JobState` (`queued`, `running`, `succeeded`,
   `failed`, `cancelled`, `interrupted`), una clave de idempotencia y cancelación cooperativa. Se
   persisten en un diario JSONL con instantánea atómica detrás de un trait `JobStore`. **No se
   adopta SQLite** en esta etapa.
3. **Un crate nuevo, `crates/chxchx-protocol`**, con los tipos serde de la API v1, compilable a
   `wasm32-unknown-unknown` y sin dependencias del sistema. No se crean crates por interfaz ni se
   reorganiza el crate actual en más crates.
4. **El cliente remoto es una PWA en Dioxus (web, WASM)** que comparte `chxchx-protocol` y que el
   nodo sirve incrustada. Android e iOS tienen la **misma prioridad** a través de la PWA
   instalable. Las apps nativas (p. ej. Dioxus mobile) quedan para un ADR posterior. **Studio de
   escritorio sigue en Iced.**
5. **Acceso remoto solo por red privada**: el nodo escucha en loopback, y la tailnet llega a él
   con `tailscale serve` (HTTPS con certificado válido, sin abrir puertos). El producto nunca
   escucha en `0.0.0.0`, nunca toca el firewall y no incluye relay público.
6. **Seguridad**: vinculación por código o QR de un solo uso, un token por dispositivo (en el nodo
   solo su hash), expiración y revocación. Roles `lector`, `operador` y `admin`, con autorización
   por acción, proyecto y dispositivo. Solo se ven proyectos **confiados**. Un cliente remoto no
   sube el nivel de acceso de ADR 0014 por encima del tope del proyecto (por defecto
   `workspace-write`), y hay un registro de auditoría. **No hay shell, PTY ni comandos
   arbitrarios remotos.**
7. **Studio usa el nodo solo si el usuario lo activa** (opt-in). Pasará a ser el comportamiento
   por defecto únicamente con otro ADR, una vez cerrada la fase H5.
8. **Manager = consolidación**: `chxchx-tech manager status|plan|apply [--dry-run]` reúne
   `doctor`, `tools install`, `local_models`, `workspace_resources` y backups. La recuperación
   automática se limita a la superficie propia del producto, y aplicar cambios al sistema no se
   expone en remoto. Las actualizaciones automáticas requieren antes un **manifiesto firmado**
   (ed25519).

## Acciones expuestas como trabajos (inventario H0)

Solo existen casos de uso que el producto ya tiene. «Local» significa que en esta etapa no se
expone en remoto.

| Caso de uso | Código actual | Tipo | Rol mínimo | Remoto |
|---|---|---|---|---|
| Estado del nodo y recursos | `workspace_resources`, `doctor` | lectura | lector | sí |
| Proyectos confiados y su resumen | `project_registry`, `project_overview` | lectura | lector | sí |
| Historial visible de chats | `chat_state`, `chat_journal` | lectura | lector | sí |
| Estado de Git (rama, cambios, historial) | `features/git` | lectura | lector | sí |
| Turno de agente (Claude, Codex, OpenCode) | `chat_bridge`, `claude_runner`, `service_runner` | trabajo | operador | sí, con tope de acceso |
| Responder una tarjeta de permiso o una pregunta | `agent_live`, `chat_bridge` | acción | operador | sí |
| Cancelar un trabajo | runner de trabajos | acción | dueño u operador | sí |
| Iniciar o detener los procesos del workspace | `workspace_lifecycle`, `workspace_processes` | trabajo | operador | sí, solo procesos configurados y confiados |
| Instalar o quitar un modelo local | `local_models` | trabajo | admin | sí |
| Diagnóstico (`doctor`, `manager status/plan`) | `doctor`, manager | lectura | admin | sí |
| `init`, `sync`, `rollback`, `integrate`, `connectors`, skills/packs/mods | `project_setup`, `mcp_*`, `project_skills`… | escritura de configuración | admin | **local** |
| Trust del proyecto | `project_trust` | seguridad | admin | **local** |
| `tools install`, `manager apply`, servicio del sistema | `tools_install`, manager | sistema | admin | **local** |
| Terminal, PTY, comando arbitrario | `integrations/terminal` | — | — | **nunca** |

## Línea base (R0.4, 2026-11-20)

La medí en el mismo equipo que la línea base del legado; los detalles están en
[PERFORMANCE-BASELINE.md](../PERFORMANCE-BASELINE.md). CLI `status`: p95 de 8 ms (fixture
pequeño) y 14 ms (grande). Studio en reposo: 28 MiB RSS y ≤ 0,3 % de CPU.

**Presupuestos del nodo**: en reposo, RSS ≤ 20 MiB y CPU ≤ 0,5 %. Respuesta a
`GET /api/v1/health` en loopback: p95 ≤ 5 ms. La feature `node` no debe cambiar las dependencias
ni el tamaño de los builds sin ella (lo comprueba la CI). El bundle de la PWA comprimido debe
quedar en ≤ 1,5 MiB y cargar en ≤ 3 s en 4G simulado. Si Dioxus no lo cumple, se reabre este ADR
antes de seguir con H4.

## Consecuencias

- La primera entrega útil (H1) no añade red: saca los trabajos de la ventana y deja `interrupted`
  en vez de perderlos. Eso ya mejora Studio por sí solo.
- Se añaden dependencias de red (`tokio`, `axum`) y de WASM (Dioxus), aisladas en su feature y en
  su crate. El build CLI/TUI/Studio no las arrastra.
- El acceso remoto depende de que el usuario tenga Tailscale. Sin él, el nodo funciona en local.
- Exponer turnos de agente con permisos de escritura a otro dispositivo es sensible: la fase H3
  no se cierra sin una revisión de seguridad.

## Fuera de alcance

Varios nodos y planificador, colaboración entre usuarios, apps nativas, relay sin VPN, ver la
pantalla remota, SQLite y aplicar cambios al sistema desde remoto. Cada uno requiere un ADR propio.
