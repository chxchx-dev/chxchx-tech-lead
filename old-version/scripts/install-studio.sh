#!/usr/bin/env bash
set -Eeuo pipefail

# URL histórica del preview: ahora delega en el instalador unificado (canal dev).
# Se mantiene para quien ya lo tenga en notas o scripts. Acepta --dry-run y --update.
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  printf 'Uso: install-studio.sh [--dry-run] [--update]\nInstala el preview de ChxChx Studio (equivale a install.sh --channel dev).\n'
  exit 0
fi
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/chxchx-dev/chxchx-tech-lead/releases/download/studio-dev-preview/install.sh \
  | bash -s -- --channel dev "$@"
