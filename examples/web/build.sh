#!/bin/bash
set -euo pipefail

# gpui-kit enables GPUI's multithreaded web feature, and its `wasm_thread` dependency uses an
# unstable std feature gate. The site runs `single_threaded_web()`, so allowing the gate on stable
# is enough to compile the unused multithreaded path.
export RUSTC_BOOTSTRAP="${RUSTC_BOOTSTRAP:-1}"

# Builds the web demo: the wasm module plus the wasm-bindgen JavaScript glue the page imports.
# Release is the default because the optimized build is ~9MB; pass --debug for a development build.
#
# Requires `rustup target add wasm32-unknown-unknown` and a `wasm-bindgen` CLI matching the
# `wasm-bindgen` crate version in Cargo.lock.

cd "$(dirname "$0")/../.."

WASM_BINDGEN="${WASM_BINDGEN:-$(command -v wasm-bindgen || echo "$HOME/.cargo/bin/wasm-bindgen")}"
if [[ ! -x "$WASM_BINDGEN" ]]; then
  echo "wasm-bindgen CLI not found; install it with: cargo install wasm-bindgen-cli" >&2
  exit 1
fi

PROFILE="release"
CARGO_ARGS=(--release)
for arg in "$@"; do
  case "$arg" in
    --debug)
      PROFILE="debug"
      CARGO_ARGS=()
      ;;
    --release)
      PROFILE="release"
      CARGO_ARGS=(--release)
      ;;
    *)
      CARGO_ARGS+=("$arg")
      ;;
  esac
done

cargo build -p gpui-scan-web --target wasm32-unknown-unknown "${CARGO_ARGS[@]}"

"$WASM_BINDGEN" "target/wasm32-unknown-unknown/$PROFILE/gpui_scan_web.wasm" \
  --out-dir examples/web/www/pkg \
  --target web \
  --no-typescript

SIZE="$(du -h examples/web/www/pkg/gpui_scan_web_bg.wasm | cut -f1)"
echo "Built examples/web/www/pkg (wasm: $SIZE) — serve it with: python3 -m http.server -d examples/web/www 4173"
