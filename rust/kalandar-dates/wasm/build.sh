#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
TOOLS="${TOOLS:?set TOOLS to a directory whose bin/ holds wasm-bindgen 0.2.121}"
nice -n 10 rustup run 1.97.0 cargo build --target wasm32-unknown-unknown --release
"$TOOLS/bin/wasm-bindgen" --target nodejs --out-dir pkg target/wasm32-unknown-unknown/release/kalandar_dates_wasm.wasm
"$TOOLS/bin/wasm-bindgen" --target web --out-dir pkg-web target/wasm32-unknown-unknown/release/kalandar_dates_wasm.wasm
printf '{"type":"commonjs"}\n' > pkg/package.json
