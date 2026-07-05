# Calendar Runtime Integration

This document captures the recommended v1 runtime integration shape for the `calendar` parser.

It is intentionally focused on runtime productization, not parser training. For the broader product shape, read:

- [Kalandar overview](../../docs/concepts/overview.md)
- [Artifacts and inference](../../docs/concepts/artifacts-and-inference.md)
- [Runtime adapters](../../docs/reference/runtime-adapters.md)
- [Tensor contract](../runtime/TENSOR_CONTRACT.md)

## Scope

This document covers:

- the developer-facing runtime API for `calendar`
- lightweight mode versus ML mode
- local artifact loading
- the inference adapter boundary
- public runtime metadata
- the pattern later `task` and `reminder` should follow

This document does not propose:

- a first-party server package
- bundling model binaries into the main npm package
- more parser training work
- turning confidence into README-style product guidance

## Current Repo State

Today the repo already has:

- a sync lightweight calendar parser via `parseEventInput(...)`
- a stable outward `EventParseResult` contract
- shared planning adapters built on top of parser-specific results
- compressed ONNX calendar artifacts with strong current metrics
- training-side helper code that proves the raw ML inference path

What is still missing is the runtime productization layer:

- a public ML parser API
- artifact loading and verification
- a local ONNX adapter
- a frozen runtime tensor boundary
- moving shipped postprocess behavior fully into runtime-owned TS code

## v1 Design Goals

1. Preserve the existing lightweight parser as the default simple path.
2. Make ML an explicit opt-in path.
3. Keep the outward event result shape stable across modes.
4. Keep local and remote inference behind the same adapter boundary.
5. Allow artifact hosting on Hugging Face, GitHub Releases, or user-owned infrastructure.
6. Reject mismatched artifacts loudly instead of silently degrading.
7. Keep the design reusable for `task` and `reminder`.

## Recommended Public API

The `calendar` package should expose two runtime entry points.

### 1. Lightweight API

Keep the existing lightweight parser as the default sync function:

```ts
import { parseEventInput } from 'kalandar/calendar';

const result = parseEventInput('design review next Tuesday at 3', {
  now: '2026-04-18T10:00:00-04:00',
  timezone: 'America/New_York',
});
```

This remains:

- sync
- zero-artifact
- zero-network
- the simplest path for normal developers

### 2. Runtime-Swappable Parser API

Add an async factory for the ML-capable runtime path:

```ts
import { createCalendarParser } from 'kalandar/calendar';

const parser = await createCalendarParser({
  mode: 'ml',
  artifact,
  inference,
});

const result = await parser.parse('design review next Tuesday at 3', {
  now: '2026-04-18T10:00:00-04:00',
  timezone: 'America/New_York',
});
```

Recommended shape:

```ts
export type CalendarParserConfig =
  | { mode?: 'lightweight' }
  | {
      mode: 'ml';
      artifact: MlArtifactSource | LoadedMlArtifact;
      inference: InferenceAdapter;
    };

export interface CalendarParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: {
    parser: 'calendar';
    suite_version?: string;
    artifact_id?: string;
  };
  parse(input: string, opts?: ParseOptions): Promise<EventParseResult>;
  dispose(): Promise<void>;
}
```

### Why this split

For v1, this is the cleanest tradeoff:

- normal developers get `parseEventInput(...)`
- advanced users get `createCalendarParser(...)`
- the default API stays sync
- ML does not leak artifact concerns into the lightweight path
- local and remote ML use the same parser contract

## Mode Model

The parser mode should stay minimal:

- `lightweight`
- `ml`

Do not make `local` and `remote` parser modes.

Those are inference backend choices, not parser modes. The parser should only care whether it is using the rule pipeline or the ML pipeline. Local ONNX versus user-owned remote execution should be expressed through the `InferenceAdapter`.

## Result Contract

Both lightweight and ML mode should return the same outward `EventParseResult` type from [./types.ts](./types.ts).

That means:

- `fields` stays the primary public payload
- `warnings` stays part of the runtime contract
- `meta.confidence` and `meta.spans` remain allowed public API
- shared planning continues to consume `EventParseResult` through `eventResultToPlanningObject(...)`

The runtime should not invent a separate ML-only event result type.

## Recommended Metadata Surface

### Public and stable

These should remain public:

- `fields`
- `warnings`
- `meta.locale`
- `meta.timezone`
- `meta.confidence`
- `meta.spans`

### Public but lightweight runtime metadata

It is worth adding a small runtime block:

```ts
meta.runtime?: {
  mode: 'lightweight' | 'ml';
  suite_version?: string;
  artifact_id?: string;
};
```

This is useful for debugging and compatibility checks without overexposing backend internals.

### Do not expose by default

Do not expose these in normal parse results:

- raw logits
- token ids
- provider-specific session details
- artifact download URLs
- full manifest contents

Those belong in advanced debug hooks, not in the ordinary parser result.

## Artifact Loading

Artifact loading should be explicit and separate from parsing.

Recommended public helper:

```ts
import { loadMlArtifact } from 'kalandar/runtime';
```

Recommended contract:

```ts
export type MlArtifactSource =
  | { kind: 'directory'; path: string }
  | { kind: 'url'; url: string }
  | { kind: 'huggingface'; repo: string; revision?: string; parser: 'calendar' }
  | { kind: 'manifest'; url: string };

export interface LoadedMlArtifact {
  parser: 'calendar';
  suite_version: string;
  artifact_id: string;
  manifest: CalendarArtifactManifest;
  tokenizer: RuntimeTokenizer;
}
```

### v1 loading rules

`loadMlArtifact(...)` should:

1. Load one atomic bundle manifest.
2. Verify `parser === 'calendar'`.
3. Verify `suite_version`.
4. Verify a single bundle-level `artifact_id`.
5. Verify required files and checksums.
6. Verify `tensor_contract_version`.
7. Verify `normalization_version`.
8. Verify `postprocess_version`.
9. Refuse partial or mismatched bundles.

This matches the repo architecture requirement that artifacts are immutable atomic bundles and must fail loudly on mismatch.

### Artifact source options

The runtime should support artifact hosting from:

- local directories
- arbitrary URLs
- Hugging Face-hosted bundles
- GitHub Releases
- user-owned internal mirrors

This keeps Kalandar client-first without requiring Kalandar to become a model hosting or inference service.

### Important v1 behavior

For v1:

- do not auto-download artifacts inside `parse(...)`
- do not silently fall back to regex when ML artifact loading fails
- do not require a persistent cache abstraction
- do not bundle large model binaries into the main npm package

If an app wants download-and-cache behavior, it can build that around `loadMlArtifact(...)`.

## Recommended Local ML Flow

The recommended local ONNX path should look like this:

```ts
import { createCalendarParser } from 'kalandar/calendar';
import { loadMlArtifact, artifactFromHuggingFace } from 'kalandar/runtime';
import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx';

const artifact = await loadMlArtifact(
  artifactFromHuggingFace({
    repo: 'your-org/kalandar-suite-2026-04',
    parser: 'calendar',
  }),
);

const parser = await createCalendarParser({
  mode: 'ml',
  artifact,
  inference: createLocalOnnxInferenceAdapter(),
});

const result = await parser.parse('Lunch with Sara every other Tuesday starting next month', {
  now: '2026-04-18T10:00:00-04:00',
  timezone: 'America/New_York',
});
```

### Why this flow

This keeps the concerns separated:

- artifact loading is explicit
- artifact verification happens once
- ONNX runtime wiring is explicit
- parsing stays simple once the parser exists

It also matches the current repo direction where client-side tokenization and postprocess are part of the runtime package rather than part of a server.

## Recommended Remote ML Flow

Kalandar should not ship a first-party inference server package.

It should, however, support user-owned remote inference through the same parser shape:

```ts
import { createCalendarParser } from 'kalandar/calendar';
import { loadMlArtifact } from 'kalandar/runtime';
import { createRemoteInferenceAdapter } from 'kalandar/runtime';

const artifact = await loadMlArtifact({
  kind: 'url',
  url: 'https://models.example.com/kalandar/suite-2026-04/calendar/manifest.json',
});

const parser = await createCalendarParser({
  mode: 'ml',
  artifact,
  inference: createRemoteInferenceAdapter({
    run: async (request) => {
      const response = await fetch('https://inference.example.com/calendar', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`Calendar inference failed: ${response.status}`);
      }

      return await response.json();
    },
  }),
});
```

This is the right v1 line:

- support user-hosted inference
- support Hugging Face or other bundle hosting
- do not ship Kalandar server software

## Inference Adapter Boundary

The adapter boundary should stop at tensors.

Kalandar runtime should own:

- input normalization
- tokenization
- truncation and padding policy
- decoding spans from logits
- field assembly into `EventParseResult`
- parser-specific postprocess
- parser warnings

The adapter should own only:

- model execution
- returning tensors in the frozen tensor contract

Recommended interface:

```ts
export interface InferenceAdapter {
  readonly kind: 'local' | 'remote' | string;
  init?(artifact: LoadedMlArtifact): Promise<void>;
  run(request: TensorRequest): Promise<TensorResponse>;
  dispose?(): Promise<void>;
}
```

### Why the boundary should be this low-level

Research-time ML helpers often treat model output as raw token classification plus ad hoc postprocess. That is useful proof that a model works, but it is not the correct public runtime boundary.

For v1, Kalandar runtime should standardize:

- tensor input/output
- entity decode
- field postprocess

That keeps:

- local ONNX consistent with remote inference
- parser semantics inside the library
- user-owned remote backends simple and backend-agnostic

## Runtime-Owned Postprocess

The repo already makes clear that final shipped behavior should not depend on training-only helper scripts.

For calendar ML mode, runtime-owned TS code should handle:

- trimming and cleanup of decoded spans
- recurrence exception structuring
- recurrence anchor cleanup
- warning generation
- contact suggestion surfacing
- any deterministic reconstruction needed to match the final `EventParseResult`

This is important because the current training artifacts and eval files still reflect raw ML spans rather than the final outward contract.

## Tensor Contract for Calendar v1

The calendar artifact currently implies a BERT-style token classification contract with:

- inputs: `input_ids`, `attention_mask`
- output: `logits`
- max sequence length: `512`

The runtime contract should freeze and document:

- input tensor names
- dtypes
- shapes
- padding strategy
- truncation behavior
- empty input behavior
- all-whitespace behavior
- oversized input behavior

This should be versioned in the artifact manifest via `tensor_contract_version`.

## Package and Export Shape

Recommended package shape:

- `kalandar/calendar`
- `kalandar/runtime`
- `kalandar/runtime/onnx`

Meaning:

- `kalandar/calendar` exports parser APIs and calendar types
- `kalandar/runtime` exports artifact loading and generic runtime contracts
- `kalandar/runtime/onnx` exports the reference local ONNX adapter

This keeps the default lightweight import path clean and lets apps opt into runtime dependencies deliberately.

## Recommended v1 Choices

### Choose

- keep `parseEventInput(...)` as the sync lightweight API
- add `createCalendarParser(...)` as the async runtime-swappable API
- keep one outward `EventParseResult` contract across modes
- make artifact loading explicit
- make inference backend pluggable
- support local ONNX and user-owned remote inference through the same adapter interface
- support artifact hosting on Hugging Face, GitHub Releases, or user-owned mirrors
- fail loudly on artifact mismatch

### Do not choose for v1

- `parseEventInput(..., { mode: 'ml' })`
- hidden artifact downloads in parse calls
- automatic regex fallback after ML load failure
- a public first-party server package
- public multi-variant model selection in the parser API

## Recommended Calendar Artifact Default

For v1, the simplest shippable choice is to publish one official calendar ML bundle per suite version.

Based on the current repo artifacts, the recommended default is the compressed `quint8_per_channel` calendar artifact:

- about `33.8 MB`
- strongest current compressed calendar metrics in the checked-in report
- acceptable current local latency in the checked-in benchmark

That gives Kalandar one obvious documented path for the calendar ML upgrade instead of exposing too many public variant choices early.

## How This Generalizes to Task and Reminder

The same runtime pattern should later apply to `task` and `reminder`:

- keep `parseTaskInput(...)` and `parseReminderInput(...)` as sync lightweight APIs
- add `createTaskParser(...)` and `createReminderParser(...)`
- reuse the same artifact loader
- reuse the same `InferenceAdapter`
- keep parser-specific postprocess and result assembly inside each parser module
- keep the shared planning layer above parser-specific results

The reusable abstraction is:

- one parser-specific sync lightweight function
- one parser-specific async factory for ML
- one shared artifact loading layer
- one shared inference adapter contract
- parser-specific result types preserved

## Open v1 Follow-Through

To make this design real, the next implementation work is:

1. Freeze the public `calendar` runtime types.
2. Move calendar ML postprocess into runtime-owned TS code.
3. Define the artifact manifest and handshake format.
4. Implement `loadMlArtifact(...)`.
5. Implement the reference local ONNX adapter.
6. Add end-to-end tests proving lightweight and ML contract compatibility.

## Summary

The recommended v1 runtime story for `calendar` is:

- lightweight stays simple and sync
- ML is opt-in and async
- artifacts are atomic external bundles
- artifacts may be hosted locally, on Hugging Face, on GitHub Releases, or on user-owned infrastructure
- inference may be local ONNX or user-owned remote execution
- Kalandar owns tokenization, decode, postprocess, and final result assembly
- Kalandar does not ship a first-party inference server

That is the simplest concrete runtime shape that matches the current repo direction and scales cleanly to `task` and `reminder`.
