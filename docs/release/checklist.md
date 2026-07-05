# Release Checklist

Use this checklist before publishing Kalandar.

## Metadata

- `package.json` has the intended `name`, `version`, `description`, `license`, repository, homepage, and issue URL.
- `package.json` does not set `private: true`.
- `rust/kalandar/Cargo.toml` has the intended name, version, description, license, repository, and readme.
- `Package.swift` builds from the package root.
- `LICENSE` exists and matches the intended license.

## Contents

- npm dry-run tarball includes `dist/`, `docs/`, selected source docs, Rust source, Swift source, and license files.
- npm dry-run tarball excludes training data, generated build output, checkpoints, local keys, and model artifacts.
- Rust crate package includes the crate source, tests, and crate README.
- SwiftPM root and nested manifests both build.

## Tests

Run:

```sh
npm run lint
npm run release:check
cargo publish --dry-run --manifest-path rust/kalandar/Cargo.toml
npm publish --dry-run --access public
npm audit --omit=dev
```

## Smoke Test

Install the packed npm tarball in a temporary project and verify:

- `import { parseCapturePreviewInput } from 'kalandar'`
- `import { createCapturePreviewParser } from 'kalandar/capture'`
- `import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx'`
- `import { createLocalCoreAiInferenceAdapter } from 'kalandar/runtime/coreai'`
- `swift build --package-path node_modules/kalandar`
- `swift build --package-path node_modules/kalandar/swift`
- `cargo build --manifest-path node_modules/kalandar/rust/Cargo.toml`

## Credentials

- `npm whoami` succeeds before real npm publish.
- crates.io token is configured before real Cargo publish.
- Git remote and tag permissions are available before the SwiftPM tag is pushed.

## What's Next

Publish with [Publishing Kalandar](./publishing.md) when every checklist item is complete.
