# Release Checklist

Use this checklist before creating a Git release of Kalandar.

## Metadata

- `package.json` has the intended `name`, `version`, `description`, `license`, repository, homepage, and issue URL.
- The Git tag or commit selected for consumers contains the intended version and documentation.
- `package.json` sets `private: true` and `rust/kalandar/Cargo.toml` sets `publish = false`.
- `Package.swift` builds from the package root.
- `LICENSE` exists and matches the intended license.

## Contents

- Git history excludes training data, generated build output, checkpoints, local keys, and Core AI model artifacts.
- The separately distributed Core AI artifact root includes matching `task`, `reminder`, and `calendar` bundles.
- `npm pack --dry-run` includes `dist/`, `docs/`, selected source docs, Rust source, Swift source, the ONNX suite in `resources/capture-parser`, and license files, while excluding Core AI model artifacts.
- The GitHub release attaches `kalandar-onnx-<version>.tar.gz`, extracted with a top-level `capture-parser/` directory, for crates.io Rust consumers.
- SwiftPM root and nested manifests both build.
- The GitHub release attaches `KalandarDatesFFI-<version>.xcframework.zip`, and `kalandarDatesChecksum` in both Swift manifests matches it.
- `src/dates/wasm/source.json` matches the WebAssembly built from the tagged `rust/kalandar-dates`.
- `rust/kalandar-dates/tools/holidays/check.sh` shows whether a newer date-holidays release exists.

## Tests

Run:

```sh
npm run lint
npm run release:check
```

For every Core AI artifact suite, run the Python Core AI smoke test and compare the optimized model outputs with the approved baseline on the parser development set. Record and approve any label drift before distributing the artifacts.

## Smoke Test

Create a temporary consuming project from the private Git dependency and verify:

- `swift build --package-path .`
- `swift test --package-path .`
- a Swift app can resolve `KalandarCoreAIRuntime` from the private Git URL
- each Core AI artifact family loads from the intended artifact root on macOS or iOS 27 or later with a Core AI-capable SDK

Also verify JavaScript imports and nested Swift and Rust builds from the Git dependency.

## Credentials

- Git remote and tag permissions are available before the private SwiftPM tag is pushed.

## What's Next

Publish with [Publishing Kalandar](./publishing.md) when every checklist item is complete.
