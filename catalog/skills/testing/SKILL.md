# Testing

- Cover observable behavior, edge cases, and failures at the layer that owns them.
- Keep tests deterministic: avoid real network calls, long-running processes, and arbitrary sleeps.
- Prefer small focused tests; use integration checks where boundaries between components matter.
- Report exactly which checks ran and whether they passed.
