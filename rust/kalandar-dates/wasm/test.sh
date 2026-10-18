#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
./build.sh
nice -n 10 cargo build --release --bin native
python3 tests/make_calls.py tests
NATIVE_BIN="$(pwd)/target/release/native" node tests/wasm_parity.mjs
