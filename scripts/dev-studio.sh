#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build_dir="${CHXCHX_BUILD_DIR:-$repo_root/build/native}"

cmake -S "$repo_root/native" -B "$build_dir"
cmake --build "$build_dir" --parallel

studio_bin="$build_dir/chxchx-studio"
if [[ ! -x "$studio_bin" && -x "$build_dir/chxchx-studio.app/Contents/MacOS/chxchx-studio" ]]; then
    studio_bin="$build_dir/chxchx-studio.app/Contents/MacOS/chxchx-studio"
fi

if [[ ! -x "$studio_bin" ]]; then
    echo "No se encontró el ejecutable de Studio en: $build_dir" >&2
    exit 1
fi

if [[ -x "$repo_root/.venv/bin/chxchx-tech" ]]; then
    export CHXCHX_TECH_CLI="$repo_root/.venv/bin/chxchx-tech"
fi

if [[ "$#" -eq 0 ]]; then
    set -- "$repo_root"
fi

exec "$studio_bin" "$@"
