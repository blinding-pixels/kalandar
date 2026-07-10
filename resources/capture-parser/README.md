# Kalandar ONNX Capture Parser Artifacts

This is the validated ONNX resource root for the Rust and Node parser paths.

- Rust: pass this directory through `--resource-root` or `KALANDAR_CAPTURE_RESOURCE_ROOT`.
- Node: load one family directory, such as `task`, `reminder`, or `calendar`, with `loadMlArtifact` and use `createLocalOnnxInferenceAdapter`.
- The suite manifest covers the complete Rust resource root. Each model family has its own JavaScript artifact manifest.

The model files require an ONNX Runtime supplied by the host: `ort` for Rust and `onnxruntime-node` for Node. Core AI artifacts are not included here.
