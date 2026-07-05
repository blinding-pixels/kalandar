# Use Swift And Core AI

This task shows how to build the Swift runtime and connect an Apple-native host to the Core AI capture path.

## Before You Begin

You need:

- Swift Package Manager.
- An Apple platform target for native usage.
- A host bridge that can return `capture-preview-v1` results.

Actual `.aimodel` execution requires an Apple SDK that provides Core AI. On older SDKs, the Swift package still builds, but forced model execution reports that Core AI is unavailable.

## Build The Swift Package From npm

Build the nested Swift package from an installed npm package:

```sh
swift build --package-path node_modules/kalandar/swift
```

## Build The Swift Package From A Git Checkout

When Kalandar is consumed as a Swift Package Manager dependency from a Git tag, the root `Package.swift` exposes the Swift products.

Build from a source checkout:

```sh
swift build --package-path .
```

## Use The SwiftPM Dependency

Add the package dependency to your Swift package:

```swift
.package(url: "https://github.com/blinding-pixels/kalandar.git", from: "0.1.0")
```

Add the library product to your target:

```swift
.product(name: "KalandarCoreAIRuntime", package: "kalandar")
```

## Use The JavaScript Bridge

JavaScript hosts select the Apple-native bridge with `implementation: "coreai"`:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser({
  implementation: 'coreai',
  bridge: {
    async previewCapture(request) {
      return nativeAppleBridge.previewCapture(request);
    },
    async dispose() {
      await nativeAppleBridge.dispose?.();
    },
  },
});
```

The bridge receives a normalized request and must return a valid `capture-preview-v1` result.

## Run App-Output Parity

From a source checkout:

```sh
npm run swift:coreai:app-parity
```

This command verifies the Swift CLI against the locked JavaScript cases and Rust golden fixtures.

## Verify Core AI Model Execution

When running on a Core AI-capable SDK, provide an artifact root and force the smoke path:

```sh
printf '%s\n' '{"text":"task: finish report tomorrow","timezone":"UTC","locale":"en-US","known_assignees":null}' \
  | KALANDAR_COREAI_ARTIFACT_ROOT=<artifact-root> \
      swift run --package-path swift kalandar-coreai-capture-preview --require-coreai-smoke
```

## What's Next

- Read [Swift and Core AI](../SWIFT.md) for package layout notes.
- Review the bridge contract in [Capture preview v1](../reference/capture-preview-v1.md).
- Inspect artifact expectations in [Core AI runtime](../../coreai/README.md).
