#!/usr/bin/env sh
set -eu

REPO="git+https://github.com/chxchx-dev/chxchx-tech-lead.git"

if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh
fi

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
uv tool install --force "$REPO"
chxchx-tech install
chxchx-tech doctor
