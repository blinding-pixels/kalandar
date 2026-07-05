# Kalandar Rust Workspace

This workspace contains the Rust capture-preview implementation for Kalandar.

The npm package exposes this implementation through `createCapturePreviewParser({ implementation: "rust-cli", ... })`. The Rust crate also publishes as `kalandar-capture-parser`.

## Before You Begin

You need:

- Rust and Cargo.
- A capture-parser resource root for resource-backed runtime behavior.

The base package does not vendor production model artifacts.

## Install

Install the CLI after the crate is published:

```sh
cargo install kalandar-capture-parser --bin kalandar-capture-preview
```

Add the library to a Rust project:

```sh
cargo add kalandar-capture-parser
```

## Build

From a source checkout:

```sh
cargo build --manifest-path rust/Cargo.toml --bin kalandar-capture-preview
```

From an installed npm package:

```sh
cargo build --manifest-path node_modules/kalandar/rust/Cargo.toml --bin kalandar-capture-preview
```

## Run

Send one request JSON object to standard input:

```sh
printf '%s\n' '{"text":"task: finish report tomorrow","timezone":"UTC","locale":"en-US","known_assignees":null}' \
  | kalandar-capture-preview --resource-root <capture-parser-resource-root>
```

The output is one `capture-preview-v1` result JSON object.

## Environment

Set `KALANDAR_CAPTURE_RESOURCE_ROOT` when the resource root is not passed as a CLI flag.

Set `KALANDAR_CAPTURE_MANIFEST_SHA256` when the host locks a specific manifest digest.

## Tests

Run the Rust tests:

```sh
cargo test --manifest-path rust/Cargo.toml
```

Resource-backed tests use `KALANDAR_CAPTURE_RESOURCE_ROOT`. When the variable is missing, those tests compile and return early.

## What's Next

- Use the CLI from JavaScript with [Use the Rust CLI](../docs/tasks/use-rust-cli.md).
- Review the release contents in [Package layout](../docs/reference/package-layout.md).
