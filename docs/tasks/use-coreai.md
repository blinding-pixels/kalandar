# Use Swift And Core AI

This task shows how to add the Swift runtime from Git and run the trained Core AI capture models in an Apple-native app.

## Before You Begin

You need:

- Swift Package Manager.
- An Apple platform target for native usage.
- Git access to `blinding-pixels/kalandar`.
- A separately distributed artifact root containing the `task`, `reminder`, and `calendar` Core AI bundles.

Actual `.aimodel` execution requires an Apple SDK that provides the `CoreAI` framework and a host running macOS or iOS 27 or later. On older SDKs or operating systems, the Swift package still builds, but model-backed parsing reports that Core AI is unavailable. A JavaScript host bridge is only needed when JavaScript, rather than Swift, owns the application boundary.

## Build The Swift Package From A Git Checkout

The root `Package.swift` exposes the Swift products for Git-based Swift Package Manager dependencies.

Build from a source checkout:

```sh
swift build --package-path .
```

## Add The SwiftPM Dependency

For an untagged private revision, use a branch or commit SHA. Replace `<git-ref>` with the revision your project has approved:

```swift
.package(url: "ssh://git@github.com/blinding-pixels/kalandar.git", revision: "<git-ref>")
```

For a tagged release, use the SSH URL with `from: "<version>"`. The tag must contain the model-backed Swift implementation.

Add the library product to your target:

```swift
.product(name: "KalandarCoreAIRuntime", package: "kalandar")
```

## Parse With A Specific Model

When the caller already knows the capture type, select the matching Core AI
artifact directly instead of relying on capture routing:

```swift
import Foundation
import KalandarCoreAIRuntime

let artifactRoot = URL(fileURLWithPath: "/path/to/kalandar-coreai-artifacts")
let suite = try CoreAIParserSuite(artifactRoot: artifactRoot)

let task = try await suite.parseTask(CapturePreviewRequest(
  text: "finish the report tomorrow at 10am"
))

print(task.value.title)
print(task.entities)
print(task.model.weightQuantization ?? "fp32")
```

`parseTask(_:)`, `parseReminder(_:)`, and `parseCalendar(_:)` each return a
typed draft, the decoded entity spans, and metadata for the model that produced
the result. Use `CoreAICapturePreviewService` when the library should route a
mixed stream of tasks, reminders, events, and thoughts first.

The direct Swift drafts also expose `leadTimeText` for reminders and
`descriptionText` for calendar events when their models identify those entities.
These are additive Swift fields; consumers of the shared capture-preview contract
can ignore them.

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

## Verify Model-Backed Parsing

When running on a Core AI-capable SDK, provide an artifact root. The service runs
the trained token-classification model for the routed parser family. Replace
`<artifact-root>` with the directory containing the three model family folders:

```sh
printf '%s\n' '{"text":"task: finish report tomorrow","timezone":"UTC","locale":"en-US","known_assignees":null}' \
  | KALANDAR_COREAI_ARTIFACT_ROOT=<artifact-root> \
      swift run --package-path swift kalandar-coreai-capture-preview --require-coreai-smoke
```

## What's Next

- Read [Swift and Core AI](../SWIFT.md) for package layout notes.
- Review the bridge contract in [Capture preview v1](../reference/capture-preview-v1.md).
- Inspect artifact expectations in [Core AI runtime](../../coreai/README.md).
