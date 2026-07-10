# Implementation Paths

Kalandar supports JavaScript, Rust, and Swift/Core AI implementation paths behind one app-facing capture contract.

## Implementations

| Implementation | Boundary | Host | Status | Best for |
| --- | --- | --- | --- | --- |
| `typescript` | In process | JavaScript | Ready | Web apps, Node apps, and fast integration. |
| `rust-cli` | JSON over standard input and standard output | Native | Ready | Desktop apps and cross-platform native hosts. |
| `coreai` | In-process Swift or native host bridge | Apple native | Artifact and SDK required for model execution | iOS, macOS, and Apple-native apps. |

## TypeScript

The TypeScript implementation runs in the same JavaScript process as your app. It is the default implementation.

Use it when:

- your app runs in a browser or Node.js
- you do not want native binaries
- you want the lowest-friction integration path

## Rust CLI

The Rust implementation is exposed as a crate and a CLI named `kalandar-capture-preview`.

Use it when:

- your app is native or desktop-first
- you want the parser outside the JavaScript process
- your app targets Windows, Linux, and macOS with the same host contract

The host sends one JSON request to standard input and reads one JSON result from standard output.

## Swift And Core AI

The Swift implementation is exposed as the `KalandarCoreAIRuntime` Swift Package Manager product.

Use it when:

- your app is Apple-native
- your host can load Swift code directly
- you can provide a Core AI artifact root
- the deployment SDK provides the `CoreAI` framework and the host runs macOS or iOS 27 or later when you need `.aimodel` execution

Use `CoreAIParserSuite` to select the task, reminder, or calendar model directly. It returns a typed Swift draft, decoded entity spans, and model metadata. Use `CoreAICapturePreviewService` to route mixed captures and return the shared `capture-preview-v1` result.

The JavaScript `coreai` implementation remains a bridge contract for JavaScript hosts. The Apple host owns the Swift runtime and returns a `capture-preview-v1` result through that bridge.

## Choosing An Implementation

Start with `typescript` unless you already know you need a native host boundary.

Move to `rust-cli` when your app needs native packaging or a cross-platform process boundary.

Use `coreai` when your product is Apple-native and can supply compatible artifacts. The Swift package builds without Core AI, but model execution requires an SDK that imports `CoreAI` and macOS or iOS 27 or later at runtime.

## Parity

The release check verifies app-output parity across the locked JavaScript, Rust, and Swift/Core AI fixture cases.

Run:

```sh
npm run release:check
```

## What's Next

- Use the default path in [Use the JavaScript parser](../tasks/use-typescript.md).
- Use the native process boundary in [Use the Rust CLI](../tasks/use-rust-cli.md).
- Use the Apple-native path in [Use Swift and Core AI](../tasks/use-coreai.md).
