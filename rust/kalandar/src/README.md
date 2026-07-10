# Capture Parser Runtime

This crate is the optional Rust-owned capture-preview parser copied into Kalandar.

Current boundary rules:

- The library exposes `CaptureParser::preview_capture`.
- The `kalandar-capture-preview` binary keeps IPC thin: one JSON request on stdin, one JSON result on stdout.
- The npm-facing TypeScript implementation remains the default parser path.
- The Rust path is explicit via `createCapturePreviewParser({ implementation: 'rust-cli', ... })`.
- The Git package vendors ONNX model artifacts at `resources/capture-parser`; callers pass that directory as the resource root.
- The `ort` dependency obtains the platform ONNX Runtime library. This package does not vendor a platform-specific ONNX Runtime binary.
