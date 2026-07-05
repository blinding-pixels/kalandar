# Package Layout

This page describes what ships in each release surface.

## npm Package

The npm package name is `kalandar`.

It includes:

- compiled JavaScript in `dist/`
- TypeScript declaration files in `dist/`
- public documentation in `README.md`, `docs/`, `coreai/README.md`, and selected source docs
- Swift source in `swift/`
- Rust source in `rust/kalandar/src/`
- Rust workspace manifests
- Core AI parity fixtures

It excludes:

- training data
- local API keys
- generated build directories
- generated model artifacts
- checkpoints

## Rust Crate

The Rust crate name is `kalandar`.

It includes:

- `kalandar` library target
- `kalandar-capture-preview` binary target
- Rust tests
- crate README

## Swift Package

The root `Package.swift` exposes Swift Package Manager products for Git tag consumption:

- `KalandarCoreAIRuntime`
- `kalandar-coreai-capture-preview`

The nested `swift/Package.swift` remains available for npm users who build from `node_modules/kalandar/swift`.

## Model Artifacts

Model artifacts are not bundled into the npm package or Rust crate.

Hosts provide model artifacts separately and validate them through runtime manifests.

## What's Next

- Verify release readiness in [Release checklist](../release/checklist.md).
- Publish when ready with [Publishing Kalandar](../release/publishing.md).
