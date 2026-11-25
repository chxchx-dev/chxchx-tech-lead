# ADR 0019: Coordinación de varias PC con un nodo maestro

- Estado: Propuesto el 2026-11-25; se acepta al empezar la fase C5 de Colmena.
- Relacionados: [ADR 0017](0017-local-node-and-remote-access.md) (H7 lo dejó fuera con «ADR propio»),
  [ADR 0018](0018-official-extensions-and-sqlite.md).

## Contexto

Con Colmena activa, varias PC con el nodo local podrían repartirse el trabajo de un mismo tablero,
cada una con su propia IA. ADR 0017 ya ofrece API v1, vinculación de dispositivos con roles y
acceso por Tailscale.

## Decisión (dirección)

1. **Un maestro por proyecto** guarda el tablero autoritativo. Los **trabajadores** se vinculan con
   el mecanismo de dispositivos existente y declaran capacidades (agentes instalados, carga, GPU).
2. **Asignación por arrendamiento.** El trabajador pide la siguiente tarjeta; el maestro se la da con
   un plazo; sin latido, el plazo vence y la tarjeta vuelve a «listo». Una tarjeta tiene a lo sumo un
   arrendatario, así que no hay edición concurrente.
3. **Sincronización por eventos** append-only con id y nodo de origen, idempotentes. Las decisiones
   resueltas viajan como `decision.resolved` al trabajador que las esperaba.
4. **Sin ejecución implícita.** Un trabajador no ejecuta nada sin trust local ni sin las tarjetas de
   permiso; una tarjeta asignada no es una orden de ejecutar procesos.
5. **Seguridad:** misma superficie que el nodo (loopback o tailnet, token por dispositivo, auditoría)
   y una revisión propia añadida a `NODE-SECURITY.md` antes de publicar C5.

## Preparación desde C1

Ids ULID, `event.origin_node`, `card.assigned_node` y un esquema que no supone un solo proceso.
