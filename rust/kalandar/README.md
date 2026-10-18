# Kalandar Capture Parser

`kalandar` is the Rust capture-preview parser runtime for Kalandar.

The crate exposes:

- the `kalandar` library
- the `kalandar-capture-preview` JSON CLI

Both surfaces use the same `capture-preview-v1` app-facing contract as the JavaScript and Swift/Core AI implementation paths.

## Install

Build the CLI from a Git checkout:

```sh
cargo build --manifest-path Cargo.toml
```

## Build

```sh
cargo build --manifest-path Cargo.toml
```

## Cargo features

`ort-download` (on by default) turns on ONNX Runtime's `download-binaries`, `copy-dylibs` and `tls-native` features. With them, the `ort` build script downloads a prebuilt ONNX Runtime at build time over TLS.

A host that provides ONNX Runtime itself, and must not download anything at build time, depends on the crate without default features:

```toml
kalandar = { version = "0.2", default-features = false }
```

`cargo tree -e features -p kalandar --no-default-features` then shows none of those features, and no TLS crate is built.

## CLI

```sh
kalandar-capture-preview --resource-root ../../resources/capture-parser < request.json
```

The CLI reads one JSON request from standard input and writes one JSON result to standard output.

## Artifacts

The Git package vendors the ONNX suite at `resources/capture-parser`. Provide that directory at runtime with one of these options:

- `--resource-root <capture-parser-resource-root>`
- `KALANDAR_CAPTURE_RESOURCE_ROOT=<capture-parser-resource-root>`

The crates.io package contains code only. Download the `kalandar-onnx-<version>.tar.gz` asset from the matching GitHub release and use its extracted `capture-parser` directory as the resource root.

Set `KALANDAR_CAPTURE_MANIFEST_SHA256` when the host locks a specific manifest digest.

## License

MIT
