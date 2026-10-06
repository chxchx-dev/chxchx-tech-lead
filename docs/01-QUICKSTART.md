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

## 2. Instalar `chxchx-tech-lead`

Instalador reducido desde Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/main/scripts/bootstrap.sh | sh
```

En Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/main/scripts/bootstrap.ps1 | iex
```

Como alternativa, instala directamente desde Git:

Desde la etiqueta estable:

```bash
uv tool install "git+https://github.com/TU_USUARIO/chxchx-tech-lead.git@v0.2.0"
chxchx-tech version
```

Cambia `TU_USUARIO` por el propietario real del repositorio. Si ya tienes el código clonado y quieres trabajar sobre él:

```bash
cd chxchx-tech-lead
uv sync --dev
uv tool install --editable .
```

## 3. Comprobar el entorno

```bash
chxchx-tech doctor
```

Los avisos sobre clientes MCP no instalados son informativos. Instala únicamente los clientes que vayas a utilizar.

## 4. Preparar el proyecto laboratorio

Cambia al repositorio de prueba y revisa primero el plan:

```bash
cd /ruta/al/proyecto-laboratorio
chxchx-tech install
chxchx-tech init --dry-run
chxchx-tech init
chxchx-tech status
```

`install` prepara Basic Memory y Serena de forma idempotente. `init` genera la configuración dentro de `.ai/`, registra el proyecto y guarda backups cuando modifica archivos gestionados.

Para una huella mínima por proyecto:

```bash
chxchx-tech init --minimal
```

Esto conserva la configuración del workspace y la memoria local sin generar los archivos de reglas y documentación opcionales.

Para ejecutar el flujo completo:

```bash
chxchx-tech setup --dry-run
chxchx-tech setup
```

## 5. Conectar un cliente MCP

Elige un cliente instalado y vuelve a revisar el plan antes de modificar su configuración:

```bash
chxchx-tech integrate --dry-run --client claude
chxchx-tech integrate --client claude
```

También están disponibles `codex` y `opencode`. Comprueba el resultado con:

```bash
chxchx-tech status
```

## 6. Pasar a proyectos reales

Cuando el laboratorio sea correcto, repite el mismo flujo en un proyecto real. Para varios proyectos registrados:

```bash
chxchx-tech projects list
chxchx-tech projects sync
```

Si necesitas volver atrás:

```bash
chxchx-tech rollback
```

Consulta [02-USAGE.md](02-USAGE.md) para perfiles, opciones de backups y detalles de cada comando.
