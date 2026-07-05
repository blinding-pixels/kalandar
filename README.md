# Kalandar

Kalandar is a natural-language capture parser for tasks, reminders, calendar events, and low-signal thoughts.

It ships three implementation paths behind one capture-preview contract:

- JavaScript for web and Node apps.
- Rust for Windows, Linux, and cross-platform native hosts.
- Swift/Core AI for Apple-native hosts.

The base npm package includes source code, runtime contracts, and integration adapters. It does not include large model artifacts, training data, checkpoints, local keys, or generated build output.

## Quick Start

Install the package:

```sh
npm install kalandar
# or
pnpm add kalandar
```

Parse a capture string:

```ts
import { parseCapturePreviewInput } from 'kalandar';

const preview = parseCapturePreviewInput('task: finish report tomorrow at 3pm #finance');

console.log(preview.kind);
console.log(preview.task);
```

Create a parser instance:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser();
const preview = await parser.parse('reminder: water plants every other saturday at 9am');
```

## Documentation

Start with [the docs index](./docs/README.md).

- [Getting started](./docs/getting-started.md): install Kalandar and parse your first capture.
- [Concepts](./docs/concepts/overview.md): understand capture preview, implementations, artifacts, and inference.
- [Tasks](./docs/tasks/use-typescript.md): use JavaScript, Rust, Core AI, or a self-hosted inference adapter.
- [Tutorials](./docs/tutorials/build-capture-preview.md): build an end-to-end capture preview flow.
- [Reference](./docs/reference/capture-preview-v1.md): inspect the `capture-preview-v1` request and response contract.
- [Publishing](./docs/release/publishing.md): verify npm, crates.io, and Swift Package Manager release readiness.

## Implementation Paths

| Path | Import or package | Best for |
| --- | --- | --- |
| JavaScript | `kalandar` and `kalandar/capture` | Web apps, Node apps, and simple integrations. |
| Rust | `kalandar-capture-parser` crate and `kalandar-capture-preview` CLI | Native desktop apps and cross-platform hosts. |
| Swift/Core AI | `KalandarCoreAIRuntime` SwiftPM product | Apple-native apps that provide a Core AI host. |

All three paths return the same `capture-preview-v1` result shape.

## Runtime Adapters

Kalandar exposes lower-level tensor runtime adapters from `kalandar/runtime`.

```ts
import { createRemoteInferenceAdapter } from 'kalandar/runtime';
import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx';
import { createLocalCoreAiInferenceAdapter } from 'kalandar/runtime/coreai';
```

Install `onnxruntime-node` in Node apps that use local ONNX inference. Apple hosts provide the Core AI bridge themselves.

## Release Checks

From a source checkout:

```sh
npm run release:check
```

That command type-checks, builds, tests Swift and Rust, runs app-output parity, and performs an npm pack dry run.

## License

MIT
