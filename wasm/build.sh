#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
TOOLS="${TOOLS:-$(cd ../.. && pwd)/.tools}"
nice -n 10 rustup run 1.97.0 cargo build --target wasm32-unknown-unknown --release
"$TOOLS/bin/wasm-bindgen" --target nodejs --out-dir pkg target/wasm32-unknown-unknown/release/chrono_english_wasm.wasm
"$TOOLS/bin/wasm-bindgen" --target web --out-dir pkg-web target/wasm32-unknown-unknown/release/chrono_english_wasm.wasm
