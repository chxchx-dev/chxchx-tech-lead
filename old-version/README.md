# old-version: el CLI de Python que aún sirve al producto Rust

El producto vive en Rust en la raíz del repositorio. De la versión anterior solo se versiona lo que aún se usa, hasta portarlo:

- `src/chxchx_tech_lead/`: el CLI de Python. Sirve el chat visual (`agent chat`), los modelos locales (`local-models`, `bridge`),
  el actualizador de Studio y los comandos que aún no existen en Rust (`run`, `ci`, `deploy`…). `chxchx-tech` los ejecuta por ti.
- `scripts/install.sh`, `scripts/install.ps1` y `scripts/release_assets.py`: los instaladores y el armado del manifiesto de cada release.
- `tests/`: sus pruebas (`uv sync && uv run pytest`).

El C++/Qt Studio, los scripts de su instalación y su CI ya no están en el repositorio (siguen en `.gitignore` por si quedan en un disco).
