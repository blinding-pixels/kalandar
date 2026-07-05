# Core AI Runtime

This directory documents the optional Apple-native implementation path.

The base npm package ships the Swift runtime source, TypeScript bridge contracts,
and parity fixtures. It does not ship `.aimodel` assets, checkpoints, training
data, or generated model artifacts.

## What Ships

- `swift/`: Swift Package Manager source for `KalandarCoreAIRuntime`.
- `kalandar/runtime/coreai`: TypeScript adapter for host-provided Apple bridges.
- `kalandar/capture`: app-facing `implementation: "coreai"` capture bridge.
- `coreai/app-parity-lock.json`: locked app-output parity cases.
- `coreai/model-parity-cases.json`: small tensor smoke cases.

## Swift Runtime

Build and test the Swift package from a source checkout:

```sh
swift test --package-path .
swift test --package-path swift
```

When installed from npm, the nested Swift package can be built with:

```sh
swift build --package-path node_modules/kalandar/swift
```

The Swift products are:

- `KalandarCoreAIRuntime`: library product for Apple-native hosts.
- `kalandar-coreai-capture-preview`: JSON-line CLI used by parity checks and host smoke tests.

## App-Facing Bridge

JavaScript apps select the Apple-native path through the capture parser factory:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser({
  implementation: 'coreai',
  bridge: nativeBridge,
});
```

The bridge receives a `capture-preview-v1` request and returns the same
`capture-preview-v1` result shape as the TypeScript and Rust implementations.

The lower-level tensor adapter is exported separately:

```ts
import { createLocalCoreAiInferenceAdapter } from 'kalandar/runtime/coreai';
```

That adapter expects a host-provided Core AI bridge. The npm package intentionally
does not depend on Apple-only Core AI libraries.

## Artifact Layout

If a native host provides Core AI model artifacts, the Swift runtime expects this
directory shape:

```text
<artifact-root>/
  task/
    manifest.json
    model.aimodel/
    tokenizer/
      vocab.txt
  reminder/
    manifest.json
    model.aimodel/
    tokenizer/
      vocab.txt
  calendar/
    manifest.json
    model.aimodel/
    tokenizer/
      vocab.txt
```

Each `manifest.json` must use `manifest_version: "kalandar-coreai-model-v1"`,
`runtime_kind: "coreai"`, `tokenizer.kind: "bert_wordpiece"`, and the tensor
names `input_ids`, `attention_mask`, and `logits`.

## Core AI Availability

The Swift package builds on older Apple SDKs because actual Core AI execution is
compiled behind `canImport(CoreAI)`. Forced `.aimodel` execution requires an Apple
SDK that provides Core AI, currently Xcode 27 / macOS or iOS 27 SDK.

On older SDKs, forced model execution reports:

```text
Core AI runtime is unavailable. Build with Xcode 27 / macOS or iOS 27 SDK.
```

## Parity Checks

App-output parity is the release gate for the Apple-native path:

```sh
npm run swift:coreai:app-parity
```

That command verifies the Swift CLI against the locked JavaScript app cases and
the Rust golden fixtures. The model-level Core AI smoke path remains SDK-gated
until a Core AI-capable Apple SDK is available.
