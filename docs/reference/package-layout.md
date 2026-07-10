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
- Swift source in `swift/`
- Rust source in `rust/kalandar/src/`
- Rust workspace manifests
- Core AI parity fixtures
- ONNX task, reminder, calendar, tokenizer, manifest, and routing artifacts in `resources/capture-parser/`

It excludes:

- training data
- local API keys
- generated build directories
- Core AI model artifacts
- checkpoints

## Rust Source Package

The Rust package identifier is `kalandar`, with publishing disabled.

It includes:

- `kalandar` library target
- `kalandar-capture-preview` binary target
- Rust tests
- crate README

It does not include the ONNX suite because crates.io has a package-size limit. Crates.io consumers use the version-matched `kalandar-onnx-<version>.tar.gz` asset attached to the GitHub release.

## Swift Package

The root `Package.swift` exposes Swift Package Manager products for Git revision or tag consumption:

- `KalandarCoreAIRuntime`
- `kalandar-coreai-capture-preview`

The nested `swift/Package.swift` remains available for npm users who build from `node_modules/kalandar/swift`.

## Core AI Model Artifacts

Core AI model artifacts are not bundled into the Git repository or Rust source package. The ONNX suite is bundled in `resources/capture-parser`.

Swift hosts provide the `task`, `reminder`, and `calendar` artifacts separately and validate them through runtime manifests. `.aimodel` execution also requires an Apple SDK that includes `CoreAI` and macOS or iOS 27 or later at runtime.

## What's Next

- Verify release readiness in [Release checklist](../release/checklist.md).
- Publish when ready with [Publishing Kalandar](../release/publishing.md).
