# Artifacts And Inference

Kalandar separates parser code from model artifact formats.

The Git repository ships parser code, runtime contracts, adapter interfaces, and a validated ONNX suite at `resources/capture-parser`. A model-backed Core AI host provides its `.aimodel` artifact bundle separately.

## Artifact Bundles

An artifact bundle is an atomic directory or remote package for one parser family.

Each bundle contains:

- model weights
- tokenizer configuration
- vocabulary
- label map
- manifest
- SHA-256 file digests
- parser, normalization, postprocess, and tensor contract versions

The runtime rejects bundles that are incomplete, mismatched, or built for a different parser contract.

## Suite Versioning

Artifacts use suite versions so related parser models can move together.

For example, a calendar model and a reminder model can belong to the same suite. A host should avoid mixing bundles from unrelated suites unless it intentionally owns that compatibility matrix.

## Local Inference

Local inference runs the model inside the host process or native runtime.

Kalandar includes:

- `createLocalOnnxInferenceAdapter` for Node hosts that install `onnxruntime-node`.
- `createLocalCoreAiInferenceAdapter` for JavaScript hosts that provide a Core AI bridge.
- `CoreAIParserSuite` and `CoreAICapturePreviewService` for native Swift hosts.

For Node, load one bundled family directory, for example `resources/capture-parser/task`. For Rust, pass the parent `resources/capture-parser` directory as the CLI resource root.

Swift Core AI inference requires an artifact root with `task`, `reminder`, and `calendar` bundles, an Apple SDK that includes `CoreAI`, and macOS or iOS 27 or later at runtime. The Swift package itself can still build on SDKs without that framework.

## Remote Inference

Remote inference lets a host tokenize locally, send tensors to user-owned infrastructure, and receive tensors back.

Kalandar does not ship a server. It defines the adapter contract so users can run their own service on infrastructure they control.

## Privacy

The default JavaScript parser runs locally. Kalandar does not send text to a remote service by default.

Remote inference is explicit. If an app sends tensors or text to external infrastructure, that app owns the privacy and data-handling policy.

## What's Next

- Build a local parser with [Use the JavaScript parser](../tasks/use-typescript.md).
- Connect a model backend with [Self-host inference](../tasks/self-host-inference.md).
- Review tensor details in [Runtime adapters](../reference/runtime-adapters.md).
