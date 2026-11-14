# Docker

- Use explicit base image versions and multi-stage builds when build tools are not needed at runtime.
- Run as a non-root user and keep secrets out of image layers and build arguments.
- Use `.dockerignore` to avoid copying credentials, caches, and unrelated build outputs.
- Keep health checks, shutdown behavior, and persistent data requirements explicit.
