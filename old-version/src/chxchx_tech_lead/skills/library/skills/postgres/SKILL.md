# PostgreSQL

- Add constraints and indexes that reflect real query patterns and domain invariants.
- Keep schema changes reversible where practical and plan migrations for existing production data.
- Parameterize queries and inspect plans for slow or high-volume access paths.
- Use transactions to keep related writes atomic; avoid holding them open across network calls.
