#!/bin/bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
crate="$(cd "$here/../.." && pwd)"
pin="$here/date-holidays.pin"
out="$crate/src/capture/holidays.bin"
samples="$crate/tests/data/holiday_samples.json"
notice="$crate/LICENSES/date-holidays.txt"

usage() {
  echo "usage: $0              regenerate from the pinned version" >&2
  echo "       $0 --bump <ver>  pin a new version, regenerate and show what changed" >&2
  exit 2
}

bump=""
case "${1:-}" in
  "") ;;
  --bump) bump="${2:-}"; [[ -n "$bump" ]] || usage ;;
  *) usage ;;
esac

pinned() { sed -n "s/^$1=//p" "$pin"; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [[ -n "$bump" ]]; then
  version="$bump"
  integrity="$(npm view "date-holidays@$version" dist.integrity)"
  [[ "$integrity" == sha512-* ]] || { echo "no sha512 integrity published for date-holidays@$version" >&2; exit 1; }
  cp "$out" "$tmp/previous.bin"
else
  version="$(pinned version)"
  integrity="$(pinned integrity)"
fi

mkdir -p "$tmp/pack" "$tmp/install"
npm pack "date-holidays@$version" --pack-destination "$tmp/pack" --silent > /dev/null
tarball="$tmp/pack/date-holidays-$version.tgz"
[[ -f "$tarball" ]] || { echo "npm pack did not produce $tarball" >&2; exit 1; }
actual="sha512-$(openssl dgst -sha512 -binary "$tarball" | openssl base64 -A)"
if [[ "$actual" != "$integrity" ]]; then
  echo "date-holidays tarball integrity mismatch: pinned $integrity, downloaded $actual" >&2
  exit 1
fi

npm install --prefix "$tmp/install" --ignore-scripts --no-save --no-audit --no-fund "$tarball"

package="$tmp/install/node_modules/date-holidays"
installed="$(node -p 'require(process.argv[1]).version' "$package/package.json")"
if [[ "$installed" != "$version" ]]; then
  echo "date-holidays version mismatch: wanted $version, installed $installed" >&2
  exit 1
fi

mkdir -p "$(dirname "$samples")"
node "$here/generate.mjs" \
  --package "$package" \
  --version "$version" \
  --integrity "$integrity" \
  --generated "$(date -u +%Y-%m-%d)" \
  --out "$out" \
  --samples "$samples" \
  --notice "$notice"

node "$here/generate_festivals.mjs" --out "$crate/src/capture/festivals.bin" --generated "$(date -u +%Y-%m-%d)"

if [[ -n "$bump" ]]; then
  printf 'version=%s\nintegrity=%s\n' "$version" "$integrity" > "$pin"
  node "$here/diff.mjs" "$tmp/previous.bin" "$out"
fi

raw=$(wc -c < "$out" | tr -d ' ')
gz=$(gzip -9 -c "$out" | wc -c | tr -d ' ')
echo "holiday table: $raw bytes raw, $gz bytes gzip -9"
