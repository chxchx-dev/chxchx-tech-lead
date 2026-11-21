# Seguridad del nodo local (H3)

- Fecha: 2026-11-20. Alcance: `chxchx-tech node` con la feature `node` y el acceso por Tailscale
  ([ADR 0017](adr/0017-local-node-and-remote-access.md), [plan](ROADMAP-HIBRIDO.md)).
- **Esto es una autorrevisión del autor del código, no una auditoría independiente.** Sirve para
  decidir qué es seguro usar hoy; antes de exponer el nodo más allá de la tailnet propia hace
  falta una revisión externa.

## Qué se protege

Un nodo con el token correcto puede hacer que un agente lea y escriba archivos de un proyecto de
confianza y, si quien lo maneja aprueba el permiso, ejecute comandos allí. Proteger el nodo es
proteger esas capacidades.

| Activo | Dónde vive | Quién lo ve |
|---|---|---|
| Token local (admin) | `node/token`, `0600` en carpeta `0700` | solo el usuario de esta PC |
| Tokens de dispositivo | en el dispositivo; en el nodo solo su SHA-256 (`devices.json`, `0600`) | el dispositivo |
| Códigos de vinculación | solo en memoria del nodo, como hash | quien los ve en la PC |
| Prompts y respuestas | `.ai/chat-state` y eventos del trabajo (con redacción de secretos) | quien lea esos archivos |
| Auditoría | `node/audit.jsonl` (`0600`) | el usuario de esta PC; un `admin` por API |

## Fronteras de confianza

1. **Procesos locales** → el nodo en `127.0.0.1`. Necesitan el token (archivo `0600`).
2. **Páginas web en el navegador de esta PC** → el nodo. Un navegador puede enviar peticiones a
   `127.0.0.1`: se rechazan por `Host` y `Origin`.
3. **Dispositivos de la tailnet** → `tailscale serve` → el nodo. Llegan desde `127.0.0.1` con el
   `Host` de la tailnet; necesitan haberse vinculado.
4. **El agente** → el proyecto. El nivel de acceso (ADR 0014) y el sandbox del proveedor mandan; el
   nodo solo decide qué nivel se le puede pedir.

## Controles y cómo se comprueban

| Riesgo | Control | Prueba |
|---|---|---|
| Entrar sin credencial | todas las rutas salvo `health` y `pair` exigen token; ninguna ruta desconocida revela nada | `every_route_that_is_not_public_refuses_a_call_without_a_token` |
| DNS rebinding y CSRF desde una web | `Host` solo `127.0.0.1`/`localhost` en el puerto del nodo, o el nombre `*.ts.net` activado; `Origin` igual | `requests_not_addressed_to_loopback_are_refused_even_with_the_token`, `the_tailscale_name_is_accepted_only_after_it_is_enabled_and_never_for_the_local_token` |
| Un dominio ajeno en `remote.json` | solo nombres `máquina.tailnet.ts.net`; un archivo dañado no autoriza nada | `only_a_tailscale_name_can_be_authorised_as_the_remote_host` |
| Usar el token local desde fuera | el token local se rechaza si la petición va al nombre de la tailnet | mismo test que arriba; comprobado con Tailscale real |
| Adivinar un código de vinculación | 50 bits, un uso, 5 min; cinco fallos cierran 10 min y cancelan los pendientes | `guessing_codes_closes_pairing_and_cancels_the_ones_waiting`, `a_pairing_code_works_once_and_guessing_it_is_stopped` |
| Robar `devices.json` | solo hashes SHA-256 de tokens de 256 bits | `only_hashes_reach_the_disk_and_devices_survive_a_restart` |
| Un dispositivo perdido | `node revoke`: el token deja de valer en la siguiente petición y se cierran sus flujos abiertos | `a_linked_device_can_only_do_what_its_role_allows`, `an_open_event_stream_is_closed_when_its_device_is_revoked` |
| Escalar privilegios | roles por ruta; parar el nodo y crear códigos solo con el token local | `a_linked_device_can_only_do_what_its_role_allows` |
| Pedir más acceso del permitido | el nivel se recorta **antes de guardar** el trabajo: tope `workspace-write` y red solo para `admin` | `a_device_cannot_raise_its_own_access_level` |
| Ver o repetir el trabajo de otro con su clave | las claves de idempotencia se ligan a quien las envía | `an_idempotency_key_belongs_to_whoever_sent_it` |
| Agotar el nodo | cuerpo de 8 KiB (320 KiB para trabajos), 128 peticiones en vuelo, 16 flujos de eventos, 50 trabajos en cola | `a_huge_body_is_refused_before_it_is_read`, `a_busy_node_refuses_new_requests_instead_of_piling_them_up`, `only_so_many_event_streams_can_be_open_at_once`, `only_so_many_jobs_may_wait_but_a_retry_of_one_that_exists_still_works` |
| Falsificar la auditoría o el log | cada texto va en una línea, sin control, acotado; nunca prompts, respuestas ni tokens | `text_cannot_forge_entries_and_is_bounded`, `the_audit_log_keeps_who_did_what_but_no_secrets`, `a_rejected_token_is_logged_without_the_token` |
| Publicar el nodo en Internet | el adapter nunca ejecuta `tailscale funnel`, rechaza un puerto con Funnel activo y no toca uno que sirva otra cosa | `the_funnel_command_cannot_be_started_at_all`, `a_port_published_on_the_internet_is_refused`, `enabling_is_planned_without_overwriting_what_is_already_served` |
| Una página que robe el token o cargue algo ajeno | política de contenido `default-src 'none'`, scripts solo propios (WebAssembly sí, `eval` y texto como código no), nada en línea, sin marcos ni formularios, y el socket solo de vuelta al host de la página | `the_policy_lets_the_page_talk_only_to_the_host_it_came_from`; comprobado en Chromium sin ningún aviso de la política |
| Respuestas del API guardadas en una caché | el API va con `no-store` y el service worker nunca lo intercepta | `every_answer_carries_the_hardening_headers_and_api_answers_are_never_cached`; en el navegador la caché solo tiene los 13 archivos de arranque |
| Servir archivos a quien no debe | solo una tabla fija incrustada al compilar, sin rutas del disco; `Host` y `Origin` como en el API | `the_web_client_is_served_without_a_token_compressed_and_with_the_right_types` |
| Identidad de la cuenta de Tailscale | Tailscale reenvía correo y nombre en `Tailscale-User-*`; el nodo no lee esas cabeceras | comprobado a mano: ningún archivo del nodo las contiene |

**Comprobado de punta a punta con Tailscale 1.104.7 real** (`e2e-remote.sh` del scratchpad):
vista previa sin cambios, `--apply` idempotente, `401` sin token, `403` con el token local o con un
`Origin` ajeno, vinculación y trabajo a través del HTTPS de la tailnet, y `disable` devolviendo
`tailscale serve` a vacío.

## Riesgos aceptados (decisiones de diseño)

1. **Un `operator` puede aprobar comandos del agente desde el celular.** Es el caso de uso. Tras
   aprobar, el agente ejecuta en un proyecto de confianza bajo el sandbox del proveedor. Quien
   vincula un dispositivo con ese rol le da esa capacidad. Para solo mirar existe `reader`.
2. **Cualquier dispositivo de la tailnet llega al nodo**, aunque sin vincularse solo ve `health`.
   Se puede acotar con las ACL de Tailscale.
3. **Un dispositivo de la tailnet puede cerrar la vinculación** durante 10 min fallando códigos a
   propósito (denegación de servicio, no acceso). Es el precio de limitar la fuerza bruta.
4. **Todos los dispositivos ven todos los trabajos.** Es un sistema de un solo usuario.
5. **Los dispositivos de la tailnet comparten el certificado HTTPS** de la máquina, cuyo nombre
   aparece en los registros públicos de transparencia de certificados.
6. **El token de dispositivo vive en `localStorage`** del navegador. Un XSS lo leería; por eso la
   política prohíbe scripts en línea y ajenos y el cliente no pinta HTML ajeno (todo el texto del
   agente va como texto, nunca como marcado). Si se añade Markdown, hay que revisarlo aquí.
7. **Los archivos del cliente son públicos** (la página carga antes de vincular): no contienen
   nada secreto, pero confirman que ahí hay un nodo.

## Abierto

- **Windows:** el token y los archivos del nodo heredan la ACL del perfil; no se probó que otro
  usuario de esa PC no pueda leerlos. Pendiente antes de ofrecer el nodo en Windows.
- **macOS:** `tailscale` de la App Store vive en `Tailscale.app/Contents/MacOS/Tailscale`; la ruta
  está prevista pero no se probó.
- **Sin prueba desde otra red** (datos móviles). Solo se probó pasando por la tailnet desde la
  propia PC. La prueba real es abrir `https://<nombre>.ts.net/api/v1/health` en el celular con
  Tailscale activo y los datos móviles encendidos.
- **Sin revisión independiente** ni *fuzzing* del API.
- **QR de vinculación:** hecho (`node pair`), con el código tras la `#` para que no llegue a ningún
  servidor, proxy ni registro. Solo con el acceso remoto activado.
- **Cliente web probado solo en Chromium:** ni Safari/iOS ni Firefox ni un teléfono real.
- **Rotación del token local:** no existe un comando; borrar `node/token` con el nodo parado
  genera otro.
- **Un `admin` remoto puede leer la auditoría y revocar dispositivos,** incluidos los de otros
  administradores. Aceptable con un solo usuario; revisar si llegan los colaboradores (H7).
