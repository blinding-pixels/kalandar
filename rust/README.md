# Kalandar Rust Workspace

This workspace contains the Rust capture-preview implementation for Kalandar:

- `kalandar`: the capture parser and the `kalandar-capture-preview` CLI.
- `kalandar-dates`: the date, time, repeat, lead-time and holiday resolver that every Kalandar implementation uses.
- `kalandar-dates-ffi`: the UniFFI binding the Swift package links.

The Git repository exposes this implementation through `createCapturePreviewParser({ implementation: "rust-cli", ... })` and as source under `rust/`.

## Before You Begin

You need:

- Rust and Cargo.
- A capture-parser resource root for resource-backed runtime behavior. The Git package provides it at `resources/capture-parser`.

The Git repository vendors the validated ONNX suite. Core AI artifacts remain separate.

The crates.io package contains the Rust code only. A crates.io consumer downloads the matching `kalandar-onnx-<version>.tar.gz` asset from the GitHub release with the same version, then passes its extracted `capture-parser` directory as the resource root.

## Install

Build the CLI from a Git checkout:

```sh
cargo build --manifest-path rust/Cargo.toml --bin kalandar-capture-preview
```

## Build

From a source checkout:

```sh
cargo build --manifest-path rust/Cargo.toml --bin kalandar-capture-preview
```

From an installed npm Git dependency:

```sh
cargo build --manifest-path node_modules/kalandar/rust/Cargo.toml --bin kalandar-capture-preview
```

## Run

Send one request JSON object to standard input:

```sh
printf '%s\n' '{"text":"task: finish report tomorrow","timezone":"UTC","locale":"en-US","known_assignees":null}' \
  | kalandar-capture-preview --resource-root resources/capture-parser
```

The output is one `capture-preview-v1` result JSON object.

## Environment

Set `KALANDAR_CAPTURE_RESOURCE_ROOT` when the resource root is not passed as a CLI flag.

The library only loads the bundled manifest digest unless the host passes its own to `CaptureParser::with_manifest_sha256`. It never reads the digest from the environment. The `kalandar-capture-preview` CLI accepts `--manifest-sha256 <digest>` or `KALANDAR_CAPTURE_MANIFEST_SHA256`.

## Tests

Run the Rust tests:

```sh
cargo test --manifest-path rust/Cargo.toml
```

Resource-backed tests use `KALANDAR_CAPTURE_RESOURCE_ROOT`. When the variable is missing, those tests compile and return early.

## What's Next

- Use the CLI from JavaScript with [Use the Rust CLI](../docs/tasks/use-rust-cli.md).
- Review the release contents in [Package layout](../docs/reference/package-layout.md).
