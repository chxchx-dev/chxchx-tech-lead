#!/usr/bin/env bash
set -Eeuo pipefail

# Compatibility URL retained for Fedora testers; the shared installer selects Fedora.
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  printf 'Uso: install-studio-fedora.sh [--dry-run]\nInstalador compartido del preview Fedora de ChxChx Studio.\n'
  exit 0
fi
curl --fail --silent --show-error --location \
  https://raw.githubusercontent.com/chxchx-dev/chxchx-tech-lead/dev/scripts/install-studio.sh \
  | bash -s -- "$@"
