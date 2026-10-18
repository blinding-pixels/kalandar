# Package Layout

This page describes what ships in each release surface.

## Git Repository

The Git repository is the private distribution. Its root `Package.swift` is
directly consumable by Swift Package Manager, and it can be installed as an npm
Git dependency. The package name remains `kalandar` after installation.

It includes:

- compiled JavaScript in `dist/`
- TypeScript declaration files in `dist/`
- public documentation in `README.md`, `docs/`, `coreai/README.md`, and selected source docs
- Swift source in `swift/`, including the generated `KalandarDates` UniFFI binding
- Rust source in `rust/kalandar/src/`, `rust/kalandar-dates/src/` and `rust/kalandar-dates-ffi/src/`
- Rust workspace manifests
- the `kalandar-dates` WebAssembly build, embedded in `dist/dates/wasm/`
- the date-holidays table `rust/kalandar-dates/src/capture/holidays.bin`, a separate data file under CC BY-SA 3.0, with its licence and attribution in `rust/kalandar-dates/LICENSES/date-holidays.txt`
- the computed festival table `rust/kalandar-dates/src/capture/festivals.bin`, under the MIT licence
- Core AI parity fixtures
- ONNX task, reminder, calendar, tokenizer, manifest, and routing artifacts in `resources/capture-parser/`

It excludes:

- training data
- local API keys
- generated build directories
- Core AI model artifacts
- checkpoints
- the `KalandarDatesFFI` XCFramework, which is a release asset

## Rust Source Package

The Rust workspace has three packages:

- `kalandar`, the capture parser.
- `kalandar-dates`, the date resolver it depends on. Its code is MIT licensed; `holidays.bin` is CC BY-SA 3.0 data.
- `kalandar-dates-ffi`, the UniFFI binding the Swift package links. It is not published.

The `kalandar` package identifier is `kalandar`, with publishing disabled.

It includes:

- `kalandar` library target
- `kalandar-capture-preview` binary target
- Rust tests
- crate README

It does not include the ONNX suite because crates.io has a package-size limit. Crates.io consumers use the version-matched `kalandar-onnx-<version>.tar.gz` asset attached to the GitHub release.

## Swift Package

The root `Package.swift` exposes Swift Package Manager products for Git revision or tag consumption:

- `KalandarCoreAIRuntime`
- `KalandarDates`
- `kalandar-coreai-capture-preview`
- `kalandar-dates-schedule`

`KalandarDates` links the `KalandarDatesFFI` binary target. When `swift/Binaries/KalandarDatesFFI.xcframework` exists, which `npm run dates:xcframework` creates, the manifest uses it. Otherwise it downloads the version-matched `KalandarDatesFFI-<version>.xcframework.zip` release asset and verifies the checksum written in `Package.swift`.

The nested `swift/Package.swift` remains available for npm users who build from `node_modules/kalandar/swift`.

## Core AI Model Artifacts

Core AI model artifacts are not bundled into the Git repository or Rust source package. The ONNX suite is bundled in `resources/capture-parser`.

Swift hosts provide the `task`, `reminder`, and `calendar` artifacts separately and validate them through runtime manifests. `.aimodel` execution also requires an Apple SDK that includes `CoreAI` and macOS or iOS 27 or later at runtime.

## What's Next

- Verify release readiness in [Release checklist](../release/checklist.md).
- Publish when ready with [Publishing Kalandar](../release/publishing.md).
