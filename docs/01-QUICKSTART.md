# 01 — Quickstart de `v0.2.0`

Esta guía instala la release estable y prepara un primer proyecto laboratorio.

## 1. Instalar `uv`

Linux y macOS:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
```

## 2. Instalar `chichan-tech-lead`

Desde la etiqueta estable:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chichan-tech-lead.git@v0.2.0"
chichan version
```

Cambia `TU_USUARIO` por el propietario real del repositorio. Si ya tienes el código clonado y quieres trabajar sobre él:

```bash
cd chichan-tech-lead
uv sync --dev
uv tool install --editable .
```

## 3. Comprobar el entorno

```bash
chichan doctor
```

Los avisos sobre clientes MCP no instalados son informativos. Instala únicamente los clientes que vayas a utilizar.

## 4. Preparar el proyecto laboratorio

Cambia al repositorio de prueba y revisa primero el plan:

```bash
cd /ruta/al/proyecto-laboratorio
chichan install
chichan init --dry-run
chichan init
chichan status
```

`install` prepara Basic Memory y Serena de forma idempotente. `init` genera la configuración dentro de `.ai/`, registra el proyecto y guarda backups cuando modifica archivos gestionados.

Para ejecutar el flujo completo:

```bash
chichan setup --dry-run
chichan setup
```

## 5. Conectar un cliente MCP

Elige un cliente instalado y vuelve a revisar el plan antes de modificar su configuración:

```bash
chichan integrate --dry-run --client claude
chichan integrate --client claude
```

También están disponibles `codex` y `opencode`. Comprueba el resultado con:

```bash
chichan status
```

## 6. Pasar a proyectos reales

Cuando el laboratorio sea correcto, repite el mismo flujo en un proyecto real. Para varios proyectos registrados:

```bash
chichan projects list
chichan projects sync
```

Si necesitas volver atrás:

```bash
chichan rollback
```

Consulta [02-USAGE.md](02-USAGE.md) para perfiles, opciones de backups y detalles de cada comando.
