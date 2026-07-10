# Kalandar

Kalandar is a natural-language capture parser for tasks, reminders, calendar events, and low-signal thoughts.

It ships three implementation paths behind one capture-preview contract:

- JavaScript for web and Node apps.
- Rust for Windows, Linux, and cross-platform native hosts.
- Swift/Core AI for Apple-native apps.

The Git repository is the distribution surface. It can be consumed as a JavaScript Git dependency or a Swift Package Manager dependency. npm registry and crates.io publication are disabled.

The repository includes parser code, runtime contracts, integration adapters, Swift source, and a validated `resources/capture-parser` ONNX suite for task, reminder, and calendar parsing. It does not include Core AI model artifacts, training data, checkpoints, local keys, or generated build output. A Core AI app supplies a validated artifact root separately.

## Quick Start

Install from the private Git repository. Replace `<git-ref>` with a commit SHA, branch, or release tag that contains the version you want to consume:

```sh
npm install git+ssh://git@github.com/blinding-pixels/kalandar.git#<git-ref>
# or
pnpm add git+ssh://git@github.com/blinding-pixels/kalandar.git#<git-ref>
```

For Apple-native apps, add the repository directly through Swift Package Manager. See [Use Swift and Core AI](./docs/tasks/use-coreai.md).

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
- [Publishing](./docs/release/publishing.md): prepare private Git tags for JavaScript and SwiftPM consumers.

## Implementation Paths

| Path | Import or package | Best for |
| --- | --- | --- |
| JavaScript | `kalandar` and `kalandar/capture` | Web apps, Node apps, and simple integrations. |
| Rust | `rust/` source and `kalandar-capture-preview` CLI | Native desktop apps and cross-platform hosts. |
| Swift/Core AI | `KalandarCoreAIRuntime` SwiftPM product | Apple-native apps that use `CoreAIParserSuite` directly or expose a JavaScript bridge. |

All three paths return the same `capture-preview-v1` result shape.

## Runtime Adapters

Kalandar exposes lower-level tensor runtime adapters from `kalandar/runtime`.

```ts
import { createRemoteInferenceAdapter } from 'kalandar/runtime';
import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx';
import { createLocalCoreAiInferenceAdapter } from 'kalandar/runtime/coreai';
```

Install `onnxruntime-node` in Node apps that use local ONNX inference. The JavaScript Core AI adapter is a bridge for Apple hosts; native Swift apps can use `CoreAIParserSuite` directly.

The bundled ONNX resources are available at `node_modules/kalandar/resources/capture-parser` after a Git dependency install. Use a family directory such as `task`, `reminder`, or `calendar` with `loadMlArtifact`; use the parent `capture-parser` directory as the Rust CLI resource root.

## Release Checks

From a source checkout:

```sh
npm run release:check
```

That command type-checks, builds, tests Swift and Rust, runs app-output parity, and performs an npm pack dry run. It does not validate separately distributed Core AI artifacts.

## License

[MIT](./LICENSE)
