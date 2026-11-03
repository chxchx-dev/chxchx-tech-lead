# ADR-0010: Modelos locales (Ollama) para OpenCode

- Estado: Aceptada (backend y pestaña de Studio)
- Fecha: 2026-11-02

## Contexto

OpenCode puede usar modelos locales a través de Ollama, pero hoy el usuario
debe elegir un modelo a ojo, instalarlo y escribir a mano un proveedor en
`opencode.json`. Además, la ventana de contexto por defecto de Ollama es de
4096 tokens, lo que rompe las llamadas a herramientas de OpenCode (su
documentación recomienda 16K–32K), y el endpoint compatible con OpenAI no
permite subirla por petición. ADR 0002 solo añadió el agente
`ollama launch opencode` y descartó expresamente detectar GPU/RAM.

## Decisión

- Python es la autoridad; Studio solo mostrará el resultado. Casos de uso en
  `integrations/` sin dependencias de UI:
  `local_hardware` (RAM, VRAM NVIDIA por `nvidia-smi`, VRAM AMD por sysfs,
  memoria unificada de Apple Silicon, disco libre), `local_model_catalog` (modelos
  con herramientas y tamaños de la biblioteca de Ollama), `local_model_fit`
  (estimación de encaje: GPU, GPU+RAM, CPU o no cabe), `ollama` (adapter),
  `opencode_local_config` y `local_models_setup` (orquestación).
- Superficies: `chxchx-tech local-models status|setup` y
  `chxchx-tech bridge local-models` (esquema `chxchx.local-models`). `setup --json`
  emite eventos `chxchx.local-models-event` (`step`, `progress`, `preview`, `done`, `error`).
- Ollama: lecturas y descargas por su API HTTP documentada (`/api/version`,
  `/api/tags`, `/api/pull` con progreso); la variante con contexto ampliado se crea con la
  CLI oficial `ollama create` y un Modelfile (`<modelo>-ctx32k`), que comparte los pesos
  del modelo base. ChxChx **no instala Ollama**: solo muestra el método oficial.
- Configuración: se fusiona en el `opencode.json` **del proyecto**, de forma aditiva e
  idempotente (solo `provider.ollama` y, con `--set-default`, `model`), con escritura
  atómica, `--dry-run` con vista previa y backup en un área propia
  (`~/.chxchx-tech-lead/backups-opencode/`). No se usa `backup_project`: su
  `restore_backup` borra los archivos gestionados ausentes del backup y un backup
  parcial haría que `rollback` eliminara `AGENTS.md` y `.ai/`. No se toca la
  configuración global de OpenCode ni se edita `opencode.jsonc`.
- La descarga exige confirmación (`--yes` o pregunta interactiva) porque gasta
  disco y ancho de banda; Studio la pedirá con tamaño y espacio libre.
- Encaje (estimación, no promesa): `pesos × 1,05 + 64 MiB por 1K de contexto`.
  Cabe en GPU si es ≤ 95 % de la VRAM; GPU+RAM si cabe sumando el 60 % de la RAM
  libre (no aplica a memoria unificada); CPU solo con modelos ≤ 10 GB y ≤ 70 % de la
  RAM libre; si no, "no cabe" (se puede forzar con `--force`).

## Consecuencias

- Un equipo sin GPU compatible recibe solo modelos pequeños y un aviso honesto de lentitud.
  Una iGPU AMD con poca VRAM reservada se detecta como GPU y puede dar "GPU+RAM"
  optimista.
- El catálogo y sus tamaños se mantienen a mano (verificados el 2026-11-02);
  conviene revisarlos al cambiar la biblioteca de Ollama.
- Una vez configurado, el modelo aparece como `ollama/<modelo>-ctx32k` en
  `opencode models` y por tanto en el selector del chat visual (ADR 0009).
- No validado con Ollama real: no estaba instalado al implementarlo. Los
  formatos de `/api/pull` y `/api/tags` siguen la documentación oficial.
- Studio: `Herramientas → Modelos locales (Ollama + OpenCode)…` (también en la paleta)
  abre una pestaña (`LocalModelsWidget`) que ejecuta `bridge local-models` y
  `local-models setup --json`; no contiene lógica de hardware, Ollama ni configuración.
  Muestra el equipo, un aviso si falta Ollama (con el comando oficial para copiar y la
  página de descarga; no lo ejecuta), la tabla con el encaje de cada modelo y la
  recomendación (★). Vista previa (`--dry-run`) sin confirmación; "Descargar y
  configurar" pide confirmación con tamaño y espacio libre y pasa `--yes` (y `--force`
  solo si el modelo no cabe, con aviso). Cerrar la pestaña con una descarga en curso
  pide confirmación; lo ya descargado se conserva en Ollama.
- Quitar: `local-models remove MODELO [--yes] [--dry-run] [--json]` borra el modelo y sus
  variantes `-ctx<N>k` por `DELETE /api/delete` (variantes primero, base al final) y los
  retira de `opencode.json` con backup. Si el `model` predeterminado apuntaba a un modelo
  quitado, se elimina (OpenCode fallaría); el proveedor `ollama` solo se borra si queda
  vacío y contiene únicamente lo que ChxChx escribió. Si el borrado en Ollama falla, no se
  toca la configuración. Solo se quitan modelos del catálogo.
- Modelos cargados: `/api/ps` (mejor esfuerzo) alimenta `ollama.loaded` y `models[].loaded`;
  Studio los muestra en el encabezado y como "En memoria" en la tabla.
- Acceso: botón "Modelos locales" en Inicio, además del menú y la paleta. El área Agentes
  no lo tiene: su barra admite cuatro acciones fijas y una quinta exige rediseñarla.
- Pendiente: acceso desde Agentes y descargar modelos que no estén en el catálogo.
