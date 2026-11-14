#!/usr/bin/env sh
set -eu

# Instala solo el comando chxchx-tech (canal estable). No configura nada más:
# cuando quieras preparar tus agentes, ejecuta `chxchx-tech install --dry-run` y luego sin --dry-run.
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.sh \
  | bash -s -- --cli-only "$@"
