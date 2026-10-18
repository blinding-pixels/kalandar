#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
rust="$root/rust"
out="$root/swift/Binaries"
name="KalandarDatesFFI"
version="$(node -p 'require(process.argv[1]).version' "$root/package.json")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cargo="${CARGO:-rustup run stable cargo}"

targets=(aarch64-apple-darwin x86_64-apple-darwin aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
for target in "${targets[@]}"; do
  nice -n 10 $cargo build --manifest-path "$rust/Cargo.toml" --release -p kalandar-dates-ffi --lib --target "$target"
done
lib() { echo "$rust/target/$1/release/libkalandar_dates_ffi.a"; }

(cd "$rust" && nice -n 10 $cargo run --release -p kalandar-dates-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "$(lib aarch64-apple-darwin)" --language swift --out-dir "$work/bindings")

mkdir -p "$work/headers" "$work/macos" "$work/ios" "$work/sim"
cp "$work/bindings/kalandar_dates_ffiFFI.h" "$work/headers/"
cp "$work/bindings/kalandar_dates_ffiFFI.modulemap" "$work/headers/module.modulemap"
lipo -create "$(lib aarch64-apple-darwin)" "$(lib x86_64-apple-darwin)" -output "$work/macos/libkalandar_dates_ffi.a"
cp "$(lib aarch64-apple-ios)" "$work/ios/libkalandar_dates_ffi.a"
lipo -create "$(lib aarch64-apple-ios-sim)" "$(lib x86_64-apple-ios)" -output "$work/sim/libkalandar_dates_ffi.a"

rm -rf "$out/$name.xcframework"
mkdir -p "$out"
xcodebuild -create-xcframework \
  -library "$work/macos/libkalandar_dates_ffi.a" -headers "$work/headers" \
  -library "$work/ios/libkalandar_dates_ffi.a" -headers "$work/headers" \
  -library "$work/sim/libkalandar_dates_ffi.a" -headers "$work/headers" \
  -output "$out/$name.xcframework"

mkdir -p "$root/swift/Sources/KalandarDates"
sed -e 's/[[:space:]]*$//' "$work/bindings/kalandar_dates_ffi.swift" > "$root/swift/Sources/KalandarDates/KalandarDatesFFI.swift"

(cd "$out" && rm -f "$name-$version.xcframework.zip" && zip -qry "$name-$version.xcframework.zip" "$name.xcframework")
checksum="$(swift package compute-checksum "$out/$name-$version.xcframework.zip")"
echo "$checksum" > "$out/$name-$version.checksum"
echo "built $out/$name.xcframework"
echo "release asset $out/$name-$version.xcframework.zip checksum $checksum"
