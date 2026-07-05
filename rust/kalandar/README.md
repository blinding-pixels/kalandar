# Kalandar Capture Parser

`kalandar` is the Rust capture-preview parser runtime for Kalandar.

The crate exposes:

- the `kalandar` library
- the `kalandar-capture-preview` JSON CLI

Both surfaces use the same `capture-preview-v1` app-facing contract as the JavaScript and Swift/Core AI implementation paths.

## Install

Install the CLI:

```sh
cargo install kalandar --bin kalandar-capture-preview
```

Add the library to a Rust project:

```sh
cargo add kalandar
```

## Build

```sh
cargo build --manifest-path Cargo.toml
```

## CLI

```sh
kalandar-capture-preview --resource-root <capture-parser-resource-root> < request.json
```

The CLI reads one JSON request from standard input and writes one JSON result to standard output.

## Artifacts

Production ONNX and resource bundles are intentionally not vendored in the crate.

Provide artifacts at runtime with one of these options:

- `--resource-root <capture-parser-resource-root>`
- `KALANDAR_CAPTURE_RESOURCE_ROOT=<capture-parser-resource-root>`

Set `KALANDAR_CAPTURE_MANIFEST_SHA256` when the host locks a specific manifest digest.

## License

MIT
