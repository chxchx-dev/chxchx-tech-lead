#!/usr/bin/env bash
set -Eeuo pipefail

# URL histórica para Fedora: el instalador unificado ya elige el paquete por la distribución.
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  printf 'Uso: install-studio-fedora.sh [--dry-run] [--update]\nEquivale a install.sh --channel dev.\n'
  exit 0
fi
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/chxchx-dev/chxchx-tech-lead/releases/download/studio-dev-preview/install.sh \
  | bash -s -- --channel dev "$@"
