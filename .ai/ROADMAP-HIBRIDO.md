# Roadmap híbrido: nodo, clientes remotos y gestión automática

- Creado: 2026-11-20. Estado: **H0 cerrada**; dirección aceptada en
  [ADR 0017](adr/0017-local-node-and-remote-access.md). Siguiente fase: H1.
- Origen: `docs/CHXCHX-Studio_Plan_Arquitectura_Hibrida_y_Automatizacion.md` (nota local del
  mantenedor). Este documento lo aterriza sobre el código actual y es el que manda.
- Relación: no sustituye a [ROADMAP.md](ROADMAP.md) ni a [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md).
  El nodo **es** el «supervisor local bajo demanda» que [ADR 0016](adr/0016-rust-control-plane.md)
  y R4.1 dejaron pendiente; este plan lo concreta y le añade acceso remoto.

## 1. Objetivo en una frase

Desde el celular o el navegador, abrir un proyecto de la PC de casa, hablar con su agente,
aprobar sus permisos, lanzar trabajos largos y recuperar el resultado aunque se corte la conexión.
Además, que el producto instale, diagnostique y repare sus propios componentes sin tocar nada fuera
de su superficie.

## 2. Qué dice la auditoría (Fase 0 del documento, respondida)

| Pregunta del documento (§13) | Respuesta en el repo hoy |
|---|---|
| ¿Qué UI usa? | **Iced 0.14**, nativa, sin WebView (ADR 0016). Un solo crate `chxchx-tech` con features `cli`, `tui`, `studio`. |
| ¿Qué hacen los servicios Python? | Uno solo: `services/agent-adapter/` conduce Codex y OpenCode en vivo. Usa solo la biblioteca estándar y tiene el contrato `chxchx.agent-adapter` v1. Se lanza **por turno** y no es un demonio. Claude corre nativo en Rust. |
| ¿Dónde se guarda el estado? | Archivos JSON/JSONL por proyecto (`.ai/chat-state`, diario, `cache/errors.jsonl`, registro de proyectos, `studio-preferences.json`) con escritura atómica y redacción de secretos. **No hay base de datos.** |
| ¿GPU, procesos largos, modelos locales? | Turnos de agentes (largos), instalación de modelos de Ollama (`local_models.rs`) y procesos del workspace. La GPU se detecta para recomendar modelos, no se gestiona. |
| ¿Sistemas operativos? | Windows, macOS, Ubuntu y Fedora, con CI en los tres. |
| ¿Colaboración o tareas remotas? | Por ahora un usuario y sus máquinas: **tareas remotas**, no edición simultánea. |

Hallazgos que cambian el diseño:

1. **El núcleo ya está separado de la UI.** Ningún archivo de `src/features/` ni de
   `src/integrations/` importa Iced, y la CI compila el build `cli` sin GUI. La «Fase 1» del
   documento está en gran parte hecha: falta exponer los casos de uso como *trabajos*, no
   separarlos.
2. **La operación representativa es el turno de agente**, con sus tarjetas de permiso (ADR 0014),
   no un cálculo genérico. Es lo que tiene valor hacer desde el celular.
3. **Hay muchas piezas del «Manager»**, solo que dispersas: `doctor`, `tools install`, el
   diagnóstico de `local_models`, `workspace_resources` (medición y presión de RAM),
   `project_backup`/`project_rollback`, `error_cache`, `studio_update` y el patrón
   preview → apply con `--dry-run` que ya usa todo el producto.
4. **Los trabajos en segundo plano viven dentro de la ventana** (`studio_jobs.rs`: un hilo y un
   callback). Al cerrar Studio se pierde el turno. Eso es lo primero que hay que resolver.
5. **El release se verifica solo con SHA-256** y el actualizador abre la página de descarga. Para
   una actualización automática hace falta antes una **firma**.

## 3. Decisiones respecto al documento original

| El documento propone | Decisión | Por qué |
|---|---|---|
| Dioxus para escritorio, web y móvil | **Iced se queda en escritorio.** El cliente remoto es una **PWA en Dioxus web (WASM)** servida por el propio nodo y con los tipos de `chxchx-protocol`. *Decidido por el mantenedor el 2026-11-20.* | Rehacer Studio no aporta nada al objetivo. La PWA cubre celular y navegador con un solo cliente, y Dioxus deja abierta la puerta a apps nativas después. |
| Apps nativas Android/iOS | **Más adelante (H7).** Primero la PWA instalable en la pantalla de inicio. | Evita firmas, tiendas y dos empaquetados antes de validar el uso. En iOS ≥ 16.4 la PWA instalada recibe notificaciones push. |
| Workspace con nueve crates | **Un solo crate nuevo, `crates/chxchx-protocol`**: tipos serde de la API, compatible con WASM y sin dependencias del sistema. El nodo es la feature `node` del crate actual. | `ROADMAP.md` prohíbe crates por interfaz y `CLAUDE.md` prohíbe reestructurar el CLI. Solo hace falta separar lo que compila a WASM. |
| SQLite para la cola | **Diario JSONL más instantánea atómica** detrás de un trait `JobStore`, como `error_cache` y `chat_state`. SQLite se revisa en H7 si llega el multi-nodo. | `rusqlite` con `bundled` añade compilación C a los tres instaladores. Un nodo con un usuario no lo necesita. |
| TLS propio y relay futuro | **TLS de Tailscale** (`tailscale serve` da `https://<pc>.<tailnet>.ts.net` con certificado válido). El nodo escucha en loopback y nunca en `0.0.0.0`. | La PWA exige HTTPS para el service worker, y así no metemos OpenSSL ni gestionamos certificados. |
| Supervisor de servicios Python persistentes | **No hace falta todavía.** El servicio Python ya es por turno; el nodo lo lanza, lo limita y lo cancela como hoy. | No hay servicios Python de larga vida. |
| Manager como pieza nueva | **Consolidar lo existente** bajo `chxchx-tech manager status\|plan\|apply`, con el patrón preview → apply de siempre. | Reutiliza código probado y mantiene las reglas de `AGENTS.md`. |
| Red de nodos y colaboración (Fase 7) | **Fuera de este plan.** Requiere un ADR propio. | `AGENTS.md` dice: «No conviertas el proyecto en un framework de agentes». El multi-nodo lo acerca a eso. |

## 4. Arquitectura objetivo

```text
  Celular / navegador (PWA)          Studio (Iced)        CLI / TUI
            │ HTTPS (tailscale serve)      │ loopback          │ loopback
            └──────────────┬───────────────┴───────────────────┘
                           ▼
                 chxchx-tech node  (feature `node`: axum + tokio)
                 ├─ auth: dispositivos vinculados, roles, auditoría
                 ├─ API v1 + eventos (WebSocket / SSE)      ← tipos de chxchx-protocol
                 ├─ JobRunner + JobStore (JSONL)            ← un dueño por proyecto
                 │    ├─ turno de agente: claude_runner (Rust) / service_runner → agent-adapter (Python)
                 │    ├─ instalación de modelo local (local_models)
                 │    └─ doctor, workspace start/stop, git status (casos de uso existentes)
                 └─ manager: inventario, plan/apply, salud, recuperación acotada, actualización firmada
```

Reglas:

- **Una sola autoridad por proyecto.** Cuando el nodo corre, él es el dueño de los turnos y
  trabajos de ese proyecto, y Studio se le conecta. Sin nodo, Studio sigue funcionando como
  hoy, dentro de su proceso. Un candado por proyecto (archivo con PID y comprobación de vida)
  impide dos dueños a la vez.
- **Lista blanca de acciones, sin comandos arbitrarios.** El nodo solo expone casos de uso que ya
  existen. No hay shell ni PTY remotos.
- **El trust local manda.** Desde fuera solo se ven y operan proyectos *confiados*. Un cliente
  remoto nunca sube el nivel de acceso de ADR 0014 por encima del tope que fija el proyecto. Por
  defecto, el tope remoto es `workspace-write`, y `full-access` remoto requiere el rol admin y
  confirmación en la PC.

## 5. Fases

Cada fase se entrega en PRs pequeños hacia `dev`, con `fmt`, `clippy -D warnings`, `cargo test`
y CI en los tres sistemas. Tamaños relativos: S ≈ 1 PR, M ≈ 2–3 PRs, L ≈ 4 o más.

### H0 — Decisiones y línea base · S

- [x] [ADR 0017](adr/0017-local-node-and-remote-access.md) con las decisiones de §3 y §4.
- [x] R0.4 en Linux: CLI `status` p95 8/14 ms, Studio en reposo 28 MiB y ≤ 0,3 % CPU
  ([PERFORMANCE-BASELINE.md](PERFORMANCE-BASELINE.md), script `scripts/perf/measure_rust.py`).
  Presupuestos del nodo: RSS ≤ 20 MiB, CPU ≤ 0,5 % y health p95 ≤ 5 ms. Pendiente: Windows,
  macOS y el arranque de Studio hasta el primer frame.
- [x] Inventario de casos de uso como trabajos, con rol mínimo y exposición remota (en ADR 0017).

**Salida:** ADR aceptado y tabla de medidas en `PERFORMANCE-BASELINE.md`. ✔

### H1 — Trabajos fuera de la ventana · M

- [x] Workspace Cargo con `crates/chxchx-protocol`: `JobId`, `JobSpec`, `JobState` (`queued`,
  `running`, `succeeded`, `failed`, `cancelled`, `interrupted`), `JobEvent`, versión de protocolo
  y errores tipados. Compila en `wasm32-unknown-unknown` en CI (job `protocol-wasm`).
  `default-members` incluye el protocolo, así que `cargo test`/`clippy` lo cubren sin flags nuevos.
  Los proyectos se identifican por su alias del registro (`ProjectId`), nunca por una ruta. Un
  `JobSpec` con campos o tipos desconocidos se rechaza, y `AccessMode::capped` aplica el tope
  remoto.
- [x] `features/jobs`: `JobStore` (instantánea `jobs.json` atómica + `journal.jsonl` en
  `<CHXCHX_TECH_HOME>/jobs`, carpeta `0700`, compacta al abrir y cada 1.000 cambios, guarda los
  200 trabajos terminados más recientes y hasta 20.000 eventos por trabajo, con redacción de
  secretos) y `JobRunner` (hilos con límite de concurrencia, cancelación cooperativa, clave de
  idempotencia, captura de pánicos y suscriptores en vivo). Un solo dueño por almacén
  (`owner.lock` con PID; un candado de un proceso muerto se retoma). Lectura sin dueño con
  `jobs::read` / `read_events` para el CLI.
- [x] Al reabrir, todo trabajo `queued` o `running` pasa a `interrupted` con su historial. Los
  encolados tampoco se reanudan solos: un agente no empieza a actuar sin nadie mirando.
- [x] Los turnos de agente corren como `JobSpec::AgentTurn` (`features/jobs/agent_turn.rs`): el
  alias se resuelve en el registro, se exige trust y el agente instalado, y el nivel se recorta
  al tope del executor. El historial va a `.ai/chat-state` como en Studio, y los permisos y
  preguntas sin nadie que responda se rechazan y se anotan.
- [x] `chxchx-tech jobs run|list|show`. **`cancel` pasa a H2**: sin nodo, un trabajo en curso
  pertenece al proceso de `jobs run` y otro proceso no tiene cómo pedirle que pare.

**Salida:** un turno lanzado desde el CLI sin Studio termina, se consulta por `job_id` y, si se
mata el proceso a la mitad, aparece como `interrupted`, nunca como perdido. ✔ Comprobado en
Linux con el binario real, un `codex` y un servicio de agentes falsos (turno completo,
`kill -9` a mitad de turno, idempotencia, alias desconocido). **H1 cerrada.**

### H2 — Nodo local · M

- [x] `chxchx-tech node start|status|stop` (feature `node`, fuera del build por defecto: `axum`
  0.8.9, `tokio` 1.54.1, `tower` 0.5.3 y `getrandom` 0.3.4, todos con versión exacta). Escucha
  solo en `127.0.0.1` y rechaza un `Host` u `Origin` ajeno (páginas web y DNS rebinding).
- [x] Token local de 256 bits en un archivo `0600` dentro de una carpeta `0700`, comparado en
  tiempo constante. **Windows: hereda la ACL del perfil del usuario, sin ACL propia; revisar
  en H3.** Un solo nodo por usuario: es el dueño del almacén de trabajos.
- [x] API v1 de trabajos: listar, crear con `Idempotency-Key`, ver, eventos desde `since`,
  cancelar y parar el nodo. Errores tipados y cuerpo limitado. 6 pruebas del API (una con
  socket real), más la prueba de punta a punta con el binario.
- [x] Presupuestos medidos en release: 10,4 MiB de RSS en reposo, 0 de CPU, health con p95 de
  1,13 ms y +1,1 MB de binario ([PERFORMANCE-BASELINE.md](PERFORMANCE-BASELINE.md)).
- [x] Eventos por WebSocket (`GET /api/v1/events[?job=ID&since=N]`, subprotocolo `chxchx.v1`;
  el token también viaja como subprotocolo `bearer.<token>`, porque un navegador no puede poner
  cabeceras en un WebSocket). La suscripción se abre antes de leer el pendiente y se descartan
  los `seq` ya enviados, así que no se pierde ni se repite nada al reconectar.
- [x] Las tarjetas de permiso y las preguntas se responden con
  `POST /api/v1/jobs/{id}/answers` (`JobAnswer`: `behavior` allow/deny con `scope` opcional, o
  `answers` para preguntas). En el nodo el turno espera hasta 10 min; sin respuesta, se rechaza.
  En `jobs run` se rechaza al instante. Cada decisión queda como evento `answered`. La petición
  se registra como pendiente antes de emitir la tarjeta, y responder dos veces o algo que no se
  espera da `409`.
- [x] `node.log` en `node/`, con rotación a 1 MiB (`node.log.1`) y archivo `0600`: arranque y
  parada, trabajos recibidos o cancelados, decisiones (nunca el texto de una respuesta) y
  peticiones rechazadas (la ruta, nunca el token).

**Salida (criterio del plan):** un cliente lanza un turno, se desconecta y, al volver, recupera
el estado y los eventos que le faltaban. ✔ Probado con un socket real (`since`) y de punta a
punta con el binario (permiso aprobado por la API mientras el turno espera). **H2 cerrada.**

**Salida (criterio del documento):** un cliente de prueba lanza un turno, se cierra y, al volver,
recupera el estado y el resultado.

### H3 — Acceso remoto seguro · M · requiere revisión de seguridad

- [ ] Integración con Tailscale en `integrations/tailscale.rs` mediante la CLI oficial. Detecta
  si está instalado y conectado, y ofrece `tailscale serve` con vista previa y deshacer. El nodo
  nunca abre puertos ni toca el firewall.
- [ ] Vinculación de dispositivos: en la PC aparece un código o QR de un solo uso (≤ 5 min) y el
  dispositivo recibe un token propio. En el nodo solo se guarda el hash del token. Los tokens
  expiran y se revocan desde Studio o el CLI.
- [ ] Roles: `lector`, `operador` y `admin` del nodo; el rol «colaborador» se aplaza. La
  autorización se comprueba por acción, proyecto y dispositivo, aunque la petición venga de la
  tailnet.
- [ ] Auditoría (`audit.jsonl`): vinculaciones, revocaciones, cambios de nivel de acceso y
  acciones de admin.
- [ ] Límites de tasa, tamaño de petición y concurrencia, y comprobación de `Origin` en la API.

**Salida:** con datos móviles, fuera del Wi‑Fi de casa, un dispositivo vinculado lanza y sigue un
turno, y uno sin vincular o revocado recibe 401 en todo.

### H4 — Cliente web / PWA · L

- [ ] Esqueleto en Dioxus web (versión 0.7.x fijada) en `apps/remote/`, con `chxchx-protocol`
  y build en CI. Se comprueba primero el presupuesto de ADR 0017: bundle comprimido ≤ 1,5 MiB y
  carga ≤ 3 s en 4G simulado. Si no se cumple, se reabre el ADR antes de construir las pantallas.
- [ ] El nodo sirve los archivos estáticos incrustados (`include_bytes!`), sin servidor extra.
- [ ] Pantallas: nodo y salud, proyectos confiados, chats (enviar, ver el streaming, aprobar o
  rechazar tarjetas, cambiar modelo), trabajos (progreso, cancelar, resultado) y dispositivos.
- [ ] Diseño táctil y responsivo, instalable como PWA, con reconexión automática que recupera el
  estado por `job_id`.
- [ ] Opcional al final: notificaciones Web Push cuando termina un turno o llega una tarjeta de
  permiso.

**Salida:** el criterio del MVP (§6) pasa en Android Chrome, iOS Safari y un navegador de
escritorio.

### H5 — Studio como cliente del nodo y servicio del sistema · M

- [ ] Si el nodo está activo, Studio le delega turnos y trabajos y se adjunta a los que siguen
  corriendo. Al cerrar la ventana no se corta nada.
- [ ] Instalación opcional del nodo como servicio **de usuario** (unidad `systemd --user`,
  `LaunchAgent` en macOS, tarea programada en Windows), con vista previa, `--dry-run` y
  desinstalación limpia. Nunca como root ni administrador.
- [ ] Studio muestra el estado del nodo y los dispositivos vinculados, con un botón para revocar.

**Salida:** se empieza un turno en la PC, se cierra Studio, se sigue desde el celular y al
reabrir Studio aparece el turno con su historial.

### H6 — Manager: instalar, vigilar y reparar · L · puede ir en paralelo desde H2

- [ ] `chxchx-tech manager status|plan|apply [--dry-run]`, que reúne `doctor`, `tools install`,
  el diagnóstico de modelos locales y la medición de recursos en un **inventario** y un **plan**
  con cambios, descargas, espacio y permisos.
- [ ] Manifiesto de componentes (binarios, servicio `agent-adapter`, herramientas opcionales)
  con versión y SHA-256. El servicio Python se recrea desde su wheel en un entorno aislado, nunca
  con `uv sync` sobre el Python del usuario.
- [ ] Supervisión: salud del nodo y del adapter, reinicios con *backoff* exponencial y *circuit
  breaker* (p. ej. ≤ 3 por hora). Después se detiene y lo explica.
- [ ] Recuperación automática solo en la lista blanca del documento (§5.5): reiniciar un proceso
  propio, limpiar cachés propias, recrear el entorno del adapter y restaurar la configuración
  previa desde su backup. Todo lo demás pide confirmación.
- [ ] **Releases firmados**: `release.yml` firma el manifiesto (ed25519/minisign) con la clave
  privada solo como secreto de CI, y el binario lleva la clave pública. Después, actualización
  aplicada en escritorio (bajar, verificar firma, drenar trabajos, reemplazar, comprobar salud y
  revertir al binario anterior si falla). En móvil manda la plataforma.
- [ ] Auto‑ajuste acotado: la concurrencia de trabajos y las sugerencias de modelo local salen de
  las medidas de `workspace_resources`, sin tocar parámetros del sistema operativo.

**Salida:** en una máquina limpia, `manager plan` describe exactamente lo que hará. `apply` lo deja
funcionando y repetirlo no cambia nada. Romper a propósito el entorno del adapter se repara solo,
y un fallo persistente se detiene y se explica.

### H7 — Futuro, con ADR propio (fuera de este plan)

Apps nativas para móvil, varios nodos con planificador por capacidades, colaboración por
proyecto, relay sin VPN, SQLite si el volumen lo pide y ver la pantalla remota (otra función,
no la API de trabajos).

## 6. MVP (H1 + H2 + H3 + una PWA mínima de H4)

> Desde el celular, por Tailscale, abrir un chat de un proyecto confiado de la PC, mandar un
> mensaje a Claude o Codex, aprobar una tarjeta de permiso, ver la respuesta en vivo, cerrar el
> navegador y, al volver, encontrar el resultado.

Pruebas de aceptación (automatizadas donde se pueda):

- [ ] El mismo turno lanzado desde Studio y desde la API produce el mismo historial en `.ai/chat-state`.
- [ ] Cerrar el cliente a mitad del turno no lo detiene, y reconectar recupera los eventos que faltaban.
- [ ] Matar el nodo a mitad del turno lo deja `interrupted` y con el historial íntegro.
- [ ] Repetir un POST con la misma `Idempotency-Key` no crea un segundo trabajo.
- [ ] Un dispositivo sin vincular, revocado o con token vencido no ve proyectos ni eventos.
- [ ] Un proyecto no confiado no aparece en remoto, y el nivel de acceso remoto no supera su tope.
- [ ] Un fallo del adapter Python llega como `failed` con su explicación y el nodo sigue vivo.
- [ ] El nodo no escucha en ninguna interfaz que no sea loopback (prueba de integración).

## 7. API v1 inicial

| Método y ruta | Uso | Rol |
|---|---|---|
| `GET /api/v1/health` | Vivo o no, sin datos sensibles | sin auth (solo loopback) |
| `GET /api/v1/node` | Versión, capacidades y recursos | lector |
| `GET /api/v1/projects` | Proyectos confiados | lector |
| `GET /api/v1/projects/{id}/chats` | Chats e historial visible | lector |
| `POST /api/v1/jobs` | Turno de agente u otra acción de la lista blanca (`Idempotency-Key`) | operador |
| `GET /api/v1/jobs/{id}` · `POST …/cancel` | Estado, resultado y cancelación | dueño u operador |
| `POST /api/v1/jobs/{id}/answers` | Responder una tarjeta de permiso o una pregunta | operador |
| `GET /api/v1/events?since=` | WebSocket filtrado por permisos | autenticado |
| `POST /api/v1/devices/pair` · `DELETE /api/v1/devices/{id}` | Vincular y revocar | admin (vincular empieza en la PC) |
| `GET /api/v1/manager/status` · `POST …/plan` | Salud y plan sin ejecutar | admin |

`POST /manager/apply` **no se expone en remoto** en este plan: aplicar cambios al sistema se hace
en la PC.

## 8. Dependencias nuevas (todas con versión exacta y detrás de su feature)

- `node`: `tokio`, `axum` (con `ws`), `tower-http` (límites), un generador de aleatorios
  criptográfico para tokens y `qrcode` para pintar el QR en Studio y en la terminal.
- `manager`: `ed25519-dalek` o `minisign-verify` para la firma.
- `crates/chxchx-protocol`: solo `serde`.
- Los builds `cli`, `tui` y `studio` sin `node` no cambian de tamaño ni de dependencias. La CI lo
  comprueba.

## 9. Riesgos

| Riesgo | Mitigación |
|---|---|
| Dos dueños del mismo chat (Studio y nodo) | Un candado por proyecto y una sola autoridad (§4), con prueba de dos modos abiertos (R4.4). |
| Acceso remoto a un agente con permisos de escritura | Solo proyectos confiados, tope remoto de nivel, tarjetas de permiso obligatorias, auditoría y revisión de seguridad en H3. |
| Tailscale no instalado o el usuario no lo quiere | El nodo funciona en local igual. Remoto sin VPN es H7 y lleva su propio análisis. |
| La PC se suspende | El cliente muestra «nodo desconectado» y el trabajo queda `interrupted`. No se cambia la energía del equipo sin consentimiento. |
| La PWA en iOS (push, fondo) | Notificaciones como mejora opcional; el estado siempre se recupera al abrir. |
| Crece el binario o el consumo en reposo | Feature `node` aparte y presupuesto medido en H0. |
| Clave de firma comprometida | Solo como secreto de CI, rotación documentada y clave pública por canal. |

## 10. Decisiones del mantenedor (2026-11-20)

1. **PWA en Dioxus web**, sujeta al presupuesto de bundle y carga de ADR 0017.
2. **Studio usa el nodo solo si el usuario lo activa** (opt-in). Pasar a «por defecto» requiere
   otro ADR tras H5.
3. **Android e iOS con la misma prioridad**, ambos vía PWA. Cada fase de cliente se prueba en
   los dos.

## 11. Primeros PRs

1. `docs`: este plan, ADR 0017, la línea base R0.4 y el enlace desde `ROADMAP.md` (H0, hecho).
2. `build`: workspace Cargo y `crates/chxchx-protocol` sin cambios de comportamiento, más la
   comprobación WASM en CI.
3. `feat(jobs)`: `JobStore` y `JobRunner` con pruebas de reinicio e idempotencia.
4. `feat(jobs)`: el turno de agente como trabajo y `chxchx-tech jobs`.
5. `feat(node)`: servidor en loopback con health, jobs y eventos.
