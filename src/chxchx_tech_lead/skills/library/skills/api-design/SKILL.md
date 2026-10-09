# API design

- Model endpoints around stable domain operations and document request and response contracts.
- Validate input at the boundary and return errors callers can handle without exposing internals.
- Make pagination, filtering, idempotency, and versioning behavior explicit where relevant.
- Keep authentication and authorization decisions consistent across equivalent operations.
