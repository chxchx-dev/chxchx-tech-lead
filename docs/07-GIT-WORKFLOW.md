# 07 — Flujo Git recomendado

## Este repositorio

```bash
git switch -c feat/<cambio>
uv run python -m compileall -q src tests scripts
uv run pytest -q
uv run python scripts/lab_smoke.py
git add .
git commit -m "feat(...): ..."
git push -u origin feat/<cambio>
```

Para publicar una versión estable, usa la guía específica de [10-PUBLISH-GIT.md](10-PUBLISH-GIT.md).

## Proyectos administrados

Para agentes en paralelo, usa worktrees o branches separados. La memoria puede ser compartida; el árbol de trabajo no.

Ejemplo:

```bash
git worktree add ../acore-codex feat/storage
git worktree add ../acore-claude review/storage
```

Evita dos agentes escribiendo simultáneamente sobre el mismo checkout.
