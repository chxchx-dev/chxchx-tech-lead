# ADR 0018: Extensiones oficiales, SQLite directo en Rust y la extensión Colmena

- Estado: Aceptado por el mantenedor el 2026-11-25.
- Fecha: 2026-11-25.
- Relacionados: [ADR 0016](0016-rust-control-plane.md), [ADR 0017](0017-local-node-and-remote-access.md),
  [ADR 0019](0019-multi-node-coordination.md), plan de trabajo en `PLAN-COLMENA.md` (local).

## Contexto

El mantenedor quiere un tablero tipo Trello dentro de Studio donde el plan de los agentes se vuelva
tarjetas, roadmap y decisiones, con un índice de búsqueda (RAG) local y la puerta abierta a
coordinar varias PC. Debe poder apagarse para que Studio siga siendo liviano. Hoy el producto no
tiene base de datos: usa JSON/JSONL con escritura atómica, y SQLite solo aparece como conector MCP
(`uvx mcp-server-sqlite`) para los agentes.

## Decisión

1. **Extensiones oficiales.** Una lista cerrada en `features/extensions.rs`, compilada en el
   binario. No hay carga dinámica de código ni extensiones de terceros. Cada extensión tiene id,
   descripción, coste declarado y un interruptor en Ajustes → Extensiones. **Apagadas por defecto.**
   Apagada no abre archivos, no arranca hilos y no muestra su icono; solo se lee un booleano.
2. **Activar es una decisión informada.** El diálogo dice qué se crea, dónde, cuánto puede ocupar y
   cómo borrarlo. Desactivar conserva los datos salvo que se pida borrarlos, y entonces se exporta
   una copia antes.
3. **SQLite directo en Rust.** `rusqlite` con `bundled` (versión exacta, FTS5 incluido) detrás de
   la feature `hive` y de un único adapter, `integrations/sqlite.rs`. Sin Python, servicios ni MCP
   para la base de la extensión. Los agentes acceden por `chxchx-tech hive …`, nunca editando el
   archivo. Modo WAL, claves foráneas activas, migraciones numeradas y una sola conexión de
   escritura por proyecto.
4. **Una base por proyecto, en los datos de la app** (`<home>/hive/<clave-de-proyecto>.db`), nunca en
   la carpeta del instalador ni dentro del repo. Instalar, actualizar o lanzar la app no la toca; una
   migración hace backup antes. El home puede cambiarse (`data_home`) y la base viaja con él.
5. **Exportación a `.ai/board.json`** automática pero **opcional por proyecto y apagada por
   defecto**, para no publicar el plan en repos públicos. La exportación manual siempre existe, con
   vista previa.
6. **Colmena no es un framework de agentes, pero no se limita.** El foco es tablero, decisiones y
   coordinación. Los agentes siguen siendo las CLI externas. Si una CLI ya ofrece orquestación
   (subagentes de Claude Code, `codex exec`, agentes de OpenCode), Colmena la aprovecha: lanza,
   observa y registra, sin reimplementarla. Esto respeta la regla de `AGENTS.md` de no convertir el
   proyecto en un framework y no cierra la puerta a integrar lo que ya existe.
7. Una decisión registrada **no concede permisos**: lo que un agente haga después sigue pasando por
   las tarjetas de permiso del ADR 0014 y por el trust local.

## Consecuencias

- Una dependencia nueva con compilación de C en los tres instaladores. Se acepta; el CI de Windows
  y macOS lo verifica desde la primera fase. El tamaño del binario se registra, no decide.
- El build `cli` sin GUI sigue compilando sin la extensión (`--no-default-features --features cli`).
- La base es estado derivado y operativo; las reglas del repositorio siguen en `AGENTS.md` y `.ai/`.

## Alternativas descartadas

- Seguir con JSON: no sirve para dependencias, búsqueda de texto ni sincronización de eventos.
- Servicio Python con su base: añade empaquetado y procesos sin necesidad.
- Sistema de plugins de terceros: superficie de seguridad y de soporte que no hace falta.
