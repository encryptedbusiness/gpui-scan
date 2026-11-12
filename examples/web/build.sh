#!/bin/bash
set -euo pipefail

# Builds the web demo: the wasm module plus the wasm-bindgen JavaScript glue the page imports.
#
# Requires `rustup target add wasm32-unknown-unknown` and a `wasm-bindgen` CLI matching the
# `wasm-bindgen` crate version in Cargo.lock.

cd "$(dirname "$0")/../.."

WASM_BINDGEN="${WASM_BINDGEN:-$(command -v wasm-bindgen || echo "$HOME/.cargo/bin/wasm-bindgen")}"
if [[ ! -x "$WASM_BINDGEN" ]]; then
  echo "wasm-bindgen CLI not found; install it with: cargo install wasm-bindgen-cli" >&2
  exit 1
fi

cargo build -p gpui-scan-web --target wasm32-unknown-unknown "$@"

PROFILE="debug"
for arg in "$@"; do
  if [[ "$arg" == "--release" ]]; then PROFILE="release"; fi
done

"$WASM_BINDGEN" "target/wasm32-unknown-unknown/$PROFILE/gpui_scan_web.wasm" \
  --out-dir examples/web/www/pkg \
  --target web \
  --no-typescript

echo "Built examples/web/www/pkg — serve it with: python3 -m http.server -d examples/web/www 4173"
