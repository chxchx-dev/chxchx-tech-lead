# Prisma

- Review generated SQL and data impact before applying a schema migration.
- Keep migrations committed and deploy them with the production migration command, not schema push.
- Select only needed fields and relations; watch for N+1 queries and unbounded result sets.
- Never expose privileged database access through client-side bundles or user-controlled raw SQL.
