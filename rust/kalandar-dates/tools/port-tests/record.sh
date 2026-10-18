#!/bin/bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
UPSTREAM="${UPSTREAM:-/Users/eshaansood/runtime-split/chrono-port/chrono-upstream}"
WORK="${WORK:-$HERE/work}"
rm -rf "$WORK" && mkdir -p "$WORK"
for f in "$UPSTREAM"/test/en/*.test.ts "$UPSTREAM"/test/system_timezone.test.ts; do
  b=$(basename "$f" .ts)
  sed -E \
    -e "s#from \"\\.\\./\\.\\./src/locales/en/parsers/ENTimeUnitCasualRelativeFormatParser\"#from \"$HERE/shim_casual_relative.mjs\"#" \
    -e "s#from \"\\.\\./\\.\\./src/locales/en/parsers/ENTimeExpressionParser\"#from \"$HERE/shim_time_exp.mjs\"#" \
    -e "s#from \"(\\.\\./)+src[^\"]*\"#from \"$HERE/shim_chrono.mjs\"#" \
    -e "s#from \"\\.\\.?/test_util\"#from \"$HERE/shim_test_util.mjs\"#" \
    "$f" > "$WORK/$b.ts"
done
cd "$WORK"
TZ=UTC CHRONO_PATH="${CHRONO_PATH:?set CHRONO_PATH to an installed chrono-node 2.10.1 directory}" \
  node --no-warnings "$HERE/recorder.mjs" "$HERE/recorded.json" "$WORK"/*.test.ts
