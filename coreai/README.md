# Core AI Runtime

This directory documents the optional Apple-native implementation path.

The Git repository ships the Swift runtime source, TypeScript bridge contracts,
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

When installed through an npm Git dependency, the nested Swift package can be built with:

```sh
swift build --package-path node_modules/kalandar/swift
```

The Swift products are:

- `KalandarCoreAIRuntime`: library product for Apple-native hosts.
- `kalandar-coreai-capture-preview`: JSON-lines CLI used by parity checks and host smoke tests. It answers each request line with one result line.

With an artifact root, `CoreAICapturePreviewService` runs the routed task,
reminder, or calendar token-classification model. It decodes BIO logits into
source entities and overlays them onto the capture draft; the rule parser remains
responsible for intent routing and temporal normalization. Without an artifact
root, the service uses the lightweight parser only. The lightweight parser is the
shared Rust rule parser, called through the `KalandarDatesFFI` binding, so its
results match the JavaScript and Rust rules.

For explicit model selection, use `CoreAIParserSuite` directly:

```swift
let suite = try CoreAIParserSuite(artifactRoot: artifactRoot)

let task = try await suite.parseTask(CapturePreviewRequest(
    text: "finish the report tomorrow at 10am"
))
print(task.value.title)
print(task.entities)
print(task.model.weightQuantization ?? "fp32")

let reminder = try await suite.parseReminder(CapturePreviewRequest(
    text: "remind me to water plants tomorrow, 15 minutes before"
))
print(reminder.value.leadTimeText)

let event = try await suite.parseCalendar(CapturePreviewRequest(
    text: "lunch with Sara tomorrow at noon; bring the draft"
))
print(event.value.descriptionText)
```

The three trained model schemas map to Swift drafts as follows. `leadTimeText` and
`descriptionText` are additive direct-Swift fields; they do not require a change
to a JavaScript or Rust `capture-preview-v1` consumer.

| Model | Swift result | Model-only fields |
| --- | --- | --- |
| `task` | `CapturePreviewTaskDraft` | title, due date/time, priority, assignee, project, tags, duration, status, recurrence |
| `reminder` | `CapturePreviewReminderDraft` | title, trigger time, lead time, location, recurrence window and exceptions |
| `calendar` | `CapturePreviewCalendarEventDraft` | title, description, start/end time, duration, location, recurrence and exceptions |

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

The Swift runtime requires a separately distributed Core AI artifact root with
this directory shape:

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

The runtime decodes the first subword piece of each whitespace word. An optional
`decoding` field selects `argmax` (the default) or `bio`, which runs constrained
BIO Viterbi. A family may also ship `sequence_decoder.json` beside its manifest,
named by the optional `sequence_decoder` field; the runtime checks that it matches
the BIO rules before use. `scripts/build-capture-parser-resources.mjs
--coreai-artifacts <artifact-root>` adds these fields to exported artifacts.

## Exporting Parser Artifacts

The package does not ship model assets. From a Kalandar training workspace (the
checkout containing `training/export_coreai_model.py`), convert each Hugging Face
checkpoint with Apple’s `coreai-torch` conversion path:

```sh
export KALANDAR_TRAINING_DATA_ROOT=/path/to/kalandar-training-data
export PYTHON=.build/coreai-export-venv/bin/python

$PYTHON training/export_coreai_model.py \
  --model-dir "$KALANDAR_TRAINING_DATA_ROOT/artifacts/task_distilled_minilm_l12_384_boundary_latest/best" \
  --output-dir package/.build/coreai-artifacts/task \
  --weight-quantization w8

$PYTHON training/export_coreai_model.py \
  --model-dir "$KALANDAR_TRAINING_DATA_ROOT/artifacts/reminder_distilled_minilm_l12_384_latest/best" \
  --output-dir package/.build/coreai-artifacts/reminder \
  --weight-quantization w8

$PYTHON training/export_coreai_model.py \
  --model-dir "$KALANDAR_TRAINING_DATA_ROOT/artifacts/calendar_distilled_minilm_l12_384_latest/best" \
  --output-dir package/.build/coreai-artifacts/calendar \
  --weight-quantization w8
```

Pass `package/.build/coreai-artifacts` as `artifactRoot` when constructing the
Swift service. `w8` is data-free INT8 per-channel weight quantization; validate
the resulting artifacts against the parser development data before adopting a
more aggressive preset. Generated assets remain outside package releases and Git
history.

## Core AI Availability

The Swift package builds on older Apple SDKs because actual Core AI execution is
compiled behind `canImport(CoreAI)`. Forced `.aimodel` execution requires an Apple
SDK that provides the `CoreAI` framework and a host running macOS or iOS 27 or
later.

On older SDKs, forced model execution reports:

```text
Core AI runtime is unavailable. Build with an SDK that includes CoreAI and run on macOS or iOS 27 or later.
```

## Parity Checks

App-output parity is the release gate for the Apple-native path:

```sh
npm run swift:coreai:app-parity
```

That command verifies the Swift CLI against the locked JavaScript app cases and
the Rust golden fixtures. Validate generated model assets separately with Apple’s
Python runtime before handing them to a Swift host.
