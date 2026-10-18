#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
nice -n 10 cargo build --release --example diff_runner
if [[ -n "${DIFFTEST_COLLECT:-}" ]]; then
  python3 difftest/collect_phrases.py difftest/phrases.json
fi
python3 difftest/build_cases.py difftest/phrases.json difftest/cases.jsonl
python3 difftest/fuzz_phrases.py difftest/phrases.json 20000 11 difftest/fuzz_phrases.json
python3 difftest/build_cases.py difftest/fuzz_phrases.json difftest/fuzz_cases.jsonl
python3 difftest/fuzz_phrases.py difftest/phrases.json 30000 23 difftest/fuzz2_phrases.json
python3 difftest/build_cases.py difftest/fuzz2_phrases.json difftest/fuzz2_cases.jsonl
python3 difftest/year_cases.py difftest/year_cases.jsonl
python3 difftest/charfuzz_phrases.py difftest/charfuzz_phrases.json
python3 difftest/build_cases.py difftest/charfuzz_phrases.json difftest/charfuzz_cases.jsonl
for name in cases fuzz_cases fuzz2_cases year_cases charfuzz_cases casefold_cases; do
  printf '%-16s ' "$name"
  python3 difftest/run_diff.py "difftest/$name.jsonl" "difftest/${name/cases/diffs}.jsonl"
done
