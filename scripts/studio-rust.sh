#!/usr/bin/env bash
# Builds and opens the Rust ChxChx Studio from this checkout (Linux/macOS).
# The legacy Qt Studio installs a launcher with the same name (`chxchx-studio`);
# this script runs the Rust binary by its explicit path so the two never get mixed up.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project="${1:-$PWD}"
cargo build --release --manifest-path "$root/migration/rust/Cargo.toml" --bin chxchx-studio
binary="$root/migration/rust/target/release/chxchx-studio"
echo "Abriendo $binary"
echo "Instalado en el PATH (puede ser el Studio Qt): $(command -v chxchx-studio || echo ninguno)"
exec "$binary" "$project"
