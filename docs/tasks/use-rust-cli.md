# Use The Rust CLI

This task shows how to use the Rust capture-preview implementation from a host app.

## Before You Begin

You need:

- Rust and Cargo.
- A Git checkout of `blinding-pixels/kalandar` or an npm Git dependency.
- A capture-parser resource root for resource-backed runtime behavior. In a checkout, use `resources/capture-parser`; after an npm Git install, use `node_modules/kalandar/resources/capture-parser`.

The Rust source is available under `rust/` in a Git checkout and under `node_modules/kalandar/rust` in an npm Git dependency.

## Build From Git

Clone the private repository, then build the CLI:

```sh
cargo build --manifest-path rust/Cargo.toml --bin kalandar-capture-preview
```

## Build The CLI From An npm Git Dependency

Build the Rust workspace from an installed npm package:

```sh
cargo build --manifest-path node_modules/kalandar/rust/Cargo.toml --bin kalandar-capture-preview
```

The binary is created under Cargo's target directory.

## Build The CLI From Source

From the package source checkout:

```sh
cargo build --manifest-path rust/Cargo.toml --bin kalandar-capture-preview
```

## Run The CLI Directly

Send one `capture-preview-v1` request on standard input:

```sh
printf '%s\n' '{"text":"task: finish report tomorrow","timezone":"UTC","locale":"en-US","known_assignees":null}' \
  | ./target/debug/kalandar-capture-preview --resource-root resources/capture-parser
```

The output is one JSON `capture-preview-v1` result.

## Use The CLI From JavaScript

Pass the command, resource root, and optional environment through the parser factory:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser({
  implementation: 'rust-cli',
  command: '/path/to/kalandar-capture-preview',
  resourceRoot: 'node_modules/kalandar/resources/capture-parser',
  timeoutMs: 30_000,
});

const preview = await parser.parse('remind me to water plants tomorrow at 9am', {
  timezone: 'America/New_York',
});
```

## Configure The Resource Root

The Rust runtime needs a resource root that contains the capture parser manifest and artifacts. The bundled root is `resources/capture-parser`.

You can pass the root in either place:

- `--resource-root <path>` when invoking the CLI directly.
- `resourceRoot` when using `createCapturePreviewParser`.

If the manifest digest is locked, set `KALANDAR_CAPTURE_MANIFEST_SHA256`.

## Troubleshooting

If the command exits with `ManifestMissing`, verify that the resource root points at the directory that contains `manifest.json`.

If the command exits with `ArtifactIntegrityMismatch`, verify that the manifest and artifact files were copied together from the same bundle.

If the JavaScript wrapper times out, increase `timeoutMs` or run the CLI directly to inspect stderr.

## What's Next

- Review the Rust package in [Package layout](../reference/package-layout.md).
- Compare implementations in [Implementation paths](../concepts/implementation-paths.md).
- Verify release readiness in [Release checklist](../release/checklist.md).
