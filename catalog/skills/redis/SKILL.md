# Redis

- Define key namespaces, value formats, and expiration behavior for every use.
- Treat cached data as disposable and keep a correct source of truth elsewhere.
- Make retries and queue handlers idempotent; plan for duplicate or delayed delivery.
- Set timeouts and bound memory usage; avoid commands that scan large keyspaces on hot paths.
