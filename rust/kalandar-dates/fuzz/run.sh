#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
SECONDS_PER_TARGET="${1:-150}"
for target in phrase_resolver recurrence_parser chrono_parse; do
  nice -n 10 rustup run nightly-2025-11-21 cargo fuzz build -O --sanitizer none "$target"
done
for target in phrase_resolver recurrence_parser chrono_parse; do
  mkdir -p "corpus/$target" "artifacts/$target"
  perl -e 'alarm shift; exec @ARGV' $((SECONDS_PER_TARGET + 60)) nice -n 10 \
    "target/aarch64-apple-darwin/release/$target" "corpus/$target" \
    -max_total_time="$SECONDS_PER_TARGET" -max_len=200 -timeout=10 \
    -artifact_prefix="artifacts/$target/" -print_final_stats=1
done
