#!/usr/bin/env sh
set -eu

if ! command -v uv >/dev/null 2>&1; then
  echo "uv no está instalado. Instalando..."
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="$HOME/.local/bin:$PATH"
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
uv tool install --force "$ROOT"

CHXCHX_BIN=$(command -v chxchx-tech || true)
if [ -z "$CHXCHX_BIN" ] && [ -x "$HOME/.local/bin/chxchx-tech" ]; then
  CHXCHX_BIN="$HOME/.local/bin/chxchx-tech"
fi

if [ -n "$CHXCHX_BIN" ]; then
  echo "Instalando herramientas gestionadas..."
  "$CHXCHX_BIN" install
else
  echo "ChxChx quedó instalado, pero no está en PATH todavía. Ejecuta: chxchx-tech install"
fi

echo "Instalado. Ejecuta: chxchx-tech doctor"
