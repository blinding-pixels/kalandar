#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
./build.sh
nice -n 10 cargo build --release --bin native
if [[ -n "${KALANDAR_MODEL_EVAL:-}" ]]; then
  python3 tests/make_calls.py tests
fi
NATIVE_BIN="$(pwd)/target/release/native" node tests/wasm_parity.mjs
