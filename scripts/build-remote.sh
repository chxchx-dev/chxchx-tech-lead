#!/usr/bin/env bash
# Builds the web client into apps/remote/dist, gzip-compressed, ready for the node to embed:
#
#     scripts/build-remote.sh && cargo build --features node
#
# Needs the wasm32 target (`rustup target add wasm32-unknown-unknown`) and wasm-bindgen's CLI at
# the exact version in Cargo.lock (`cargo install wasm-bindgen-cli --version =0.2.130 --locked`).
# `wasm-opt` is used when it is installed. Fails if the compressed app is over the budget of
# ADR 0017 (1.5 MiB; override with CHXCHX_REMOTE_BUDGET_BYTES).
set -euo pipefail
cd "$(dirname "$0")/.."

BUDGET=${CHXCHX_REMOTE_BUDGET_BYTES:-1572864}
TARGET=wasm32-unknown-unknown
OUT=apps/remote/dist

wanted=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[^0-9.]/, "", $0); print; exit}' Cargo.lock)
have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ "$have" != "$wanted" ]; then
  echo "wasm-bindgen CLI is '${have:-missing}', Cargo.lock wants $wanted:" >&2
  echo "  cargo install wasm-bindgen-cli --version =$wanted --locked" >&2
  exit 1
fi

cargo build --locked -p chxchx-remote --target "$TARGET" --profile wasm-release

rm -rf "$OUT"
mkdir -p "$OUT/pkg"
wasm-bindgen --target web --no-typescript --out-dir "$OUT/pkg" --out-name app \
  "target/$TARGET/wasm-release/chxchx-remote.wasm"
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz --all-features "$OUT/pkg/app_bg.wasm" -o "$OUT/pkg/app_bg.wasm"
fi
cp -r apps/remote/web/. "$OUT/"
python3 -I scripts/finish_remote.py "$OUT"

# A worker that does not even parse is never registered and fails silently in the browser.
if command -v node >/dev/null 2>&1; then
  node --check "$OUT/sw.js"
fi

# `gzip -n` leaves out the name and time, so the same build gives the same bytes.
find "$OUT" -type f ! -name '*.gz' -print0 | while IFS= read -r -d '' file; do
  gzip -9 -n -c "$file" > "$file.gz"
done

total=$(find "$OUT" -name '*.gz' -printf '%s\n' | awk '{s += $1} END {print s + 0}')
wasm=$(stat -c %s "$OUT/pkg/app_bg.wasm.gz")
echo "web client: $total bytes compressed in total, app_bg.wasm $wasm (budget $BUDGET)"
if [ "$total" -gt "$BUDGET" ]; then
  echo "over the budget of ADR 0017: reopen it before adding more" >&2
  exit 1
fi
