#!/bin/bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
version="$(sed -n 's/^version=//p' "$here/date-holidays.pin")"
times="$(npm view date-holidays time --json)"
latest="$(npm view date-holidays dist-tags.latest)"
node -e '
const [times, version, latest] = [JSON.parse(process.argv[1]), process.argv[2], process.argv[3]];
const days = (v) => Math.floor((Date.now() - Date.parse(times[v])) / 86400000);
console.log(`pinned date-holidays ${version}, published ${times[version].slice(0, 10)}, ${days(version)} days old`);
if (latest !== version) {
  console.log(`newer version available: ${latest}, published ${times[latest].slice(0, 10)}`);
  console.log("review it with: tools/holidays/regenerate.sh --bump " + latest);
  process.exitCode = 1;
} else {
  console.log("this is the latest published version");
}
' "$times" "$version" "$latest"
