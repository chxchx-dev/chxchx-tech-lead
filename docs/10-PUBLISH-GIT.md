# 10 — Publicar e instalar desde Git

Esta guía publica `v0.2.0` como release estable. Sustituye la URL de ejemplo por el repositorio real.

## 1. Preparar el repositorio

Desde la raíz del proyecto:

```bash
git status
```

Si todavía no existe un repositorio Git, inicialízalo una sola vez:

```bash
git init
git branch -M main
git remote add origin https://github.com/TU_USUARIO/chxchx-tech-lead.git
```

## 2. Verificar el contenido

```bash
uv run python -m compileall -q src tests scripts
uv run pytest -q
uv run python scripts/lab_smoke.py
uv lock --check
```

No publiques `.env`, tokens, credenciales ni archivos de configuración personal.

## 3. Crear la release

```bash
git add .
git commit -m "release: chxchx-tech-lead v0.2.0"
git tag -a v0.2.0 -m "chxchx-tech-lead v0.2.0"
git push -u origin main
git push origin v0.2.0
```

La etiqueta debe ser exactamente `v0.2.0`, porque es la referencia que usará `uv` en la instalación estable.

## 4. Instalar la release

En cualquier equipo con `uv`:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
chxchx-tech version
chxchx-tech doctor
```

Para reinstalar una herramienta ya instalada:

```bash
uv tool install --force "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
```

## 5. Publicar una actualización

Para una nueva versión, actualiza `pyproject.toml`, `src/chxchx_tech_lead/__init__.py` y `CHANGELOG.md`; después crea una etiqueta nueva, por ejemplo `v0.2.1`:

```bash
git add .
git commit -m "release: chxchx-tech-lead v0.2.1"
git tag -a v0.2.1 -m "chxchx-tech-lead v0.2.1"
git push origin main
git push origin v0.2.1
```

La etiqueta publicada no debe reutilizarse para contenido diferente. Si una release necesita corrección, publica una nueva versión.
