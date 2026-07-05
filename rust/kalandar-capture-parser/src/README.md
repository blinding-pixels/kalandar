# Capture Parser Runtime

This crate is the optional Rust-owned capture-preview parser copied into Kalandar.

Current boundary rules:

- The library exposes `CaptureParser::preview_capture`.
- The `kalandar-capture-preview` binary keeps IPC thin: one JSON request on stdin, one JSON result on stdout.
- The npm-facing TypeScript implementation remains the default parser path.
- The Rust path is explicit via `createCapturePreviewParser({ implementation: 'rust-cli', ... })`.
- ONNX model artifacts and ONNX Runtime archives are caller-provided through a resource root; they are not vendored in this package.
- Cargo linking requires `ORT_LIB_PATH` or `ORT_LIB_LOCATION` to point at the resource root's ONNX Runtime library directory before building binaries or running resource-backed tests. When that directory contains dynamic libraries, set `ORT_PREFER_DYNAMIC_LINK=1` too.
