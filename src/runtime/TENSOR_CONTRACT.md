# Kalandar Runtime Tensor Contract

This document freezes the current tensor boundary for Kalandar v1 runtime inference. It is the repo-specific contract for `calendar`, `task`, and `reminder` ML parsing as implemented today in:

- `runtime/types.ts`
- `runtime/artifacts.ts`
- `runtime/tokenizer.ts`
- `calendar/ml.ts`
- `task/ml.ts`
- `reminder/ml.ts`

This is a runtime integration document, not a training document.

## Scope

Kalandar supports:

- local inference through a user-supplied/local `InferenceAdapter`
- remote inference through a user-owned `InferenceAdapter`

Kalandar does not ship a first-party inference server package or hosted inference API.

Tokenization and postprocess are client-side/runtime-side responsibilities. A remote adapter receives tensors plus the parser/artifact compatibility metadata needed to validate the request, and returns tensors.

## Current parser contract versions

Every artifact bundle must declare one of these parser-specific tensor contract versions:

| Parser | `tensor_contract_version` | `normalization_version` | `postprocess_version` |
| --- | --- | --- | --- |
| `calendar` | `calendar-bert-token-classification-v1` | `calendar-bert-uncased-v1` | `calendar-event-postprocess-v1` |
| `task` | `task-bert-token-classification-v1` | `task-bert-uncased-v1` | `task-postprocess-v1` |
| `reminder` | `reminder-bert-token-classification-v1` | `reminder-bert-uncased-v1` | `reminder-postprocess-v1` |

The runtime validates these values during artifact load before any adapter is used.

## Tensor inputs

All three ML parsers currently send the same tensor names and dtypes.

| Tensor | Dtype | Rank | Shape | Meaning |
| --- | --- | --- | --- | --- |
| `input_ids` | `int64` | `2` | `[1, sequence_length]` | Client-tokenized BERT WordPiece ids, including `[CLS]` and `[SEP]` |
| `attention_mask` | `int64` | `2` | `[1, sequence_length]` | All `1`s for the transmitted sequence; Kalandar does not send padded positions |

Shape expectations:

- `sequence_length` is variable per request.
- Parser inference requests include `[CLS]`, at least one non-special token, and `[SEP]`, so `3 <= sequence_length <= 512` for normal tensor requests. Empty or whitespace-only parser input returns an empty parser result before any tensor request is issued.
- `sequence_length` must match across `input_ids`, `attention_mask`, and the returned `logits`.
- Kalandar sends batch size `1` only.

## Tensor outputs

All three ML parsers currently request one output tensor.

| Tensor | Dtype | Rank | Shape | Meaning |
| --- | --- | --- | --- | --- |
| `logits` | `float32` | `3` | `[1, sequence_length, label_count]` | Per-token classifier logits returned by the model |

Output expectations:

- batch size must be `1`
- `sequence_length` must equal the input tensor length for that request
- `label_count` must equal the loaded artifact's label map size

Current repo/test-bundle label dimensions are:

- `calendar`: 19 required labels
- `task`: 27 required labels
- `reminder`: 19 required labels

The runtime's hard requirement is consistency with the artifact's `config.json` label map plus the required label set for that parser.

## Max sequence length, padding, and truncation

Current runtime behavior is fixed by `runtime/artifacts.ts` and `runtime/tokenizer.ts`:

- max sequence length: `512`
- padding: `none`
- truncation: `tail`

Concrete meaning:

- Kalandar keeps the actual encoded length; it does not pad requests up to `512`.
- `attention_mask` is therefore the same length as `input_ids` and is all `1`s.
- The tokenizer reserves room for `[CLS]` and `[SEP]`, so at most `510` non-special wordpieces are kept.
- If the input is oversized, Kalandar truncates before calling `InferenceAdapter.run(...)`.

## Empty, whitespace-only, and oversized input behavior

The runtime contract is intentionally explicit here.

### Empty input

- Manifest contract: `empty_input_behavior = return_empty_result`
- Runtime behavior: if the normalized parser input is empty, Kalandar does not call inference
- Result behavior: the parser returns its normal empty-result shape directly

### Whitespace-only input

- Manifest contract: `whitespace_input_behavior = return_empty_result`
- Runtime behavior: parser contexts normalize and trim whitespace before inference
- Result behavior: whitespace-only input is treated the same as empty input and does not cross the tensor boundary

### Oversized input

- Manifest contract: `oversized_input_behavior = truncate`
- Runtime behavior: Kalandar truncates in the tokenizer before calling local or remote inference
- Result behavior: parsing continues on the truncated sequence and the outward parse result gets an `ml_input_truncated` warning
- Remote boundary implication: a remote adapter only sees already-truncated tensors; it must not re-tokenize or assume a fixed `512`-length padded shape

## Artifact handshake expectations

Remote inference is only stable if the runtime and the user-owned inference service agree on the exact artifact bundle.

At runtime:

- `loadMlArtifact(...)` validates the artifact bundle locally before inference is used
- the bundle must be atomic and checksum-verified
- the runtime rejects parser mismatches, bad checksums, missing files, unsupported contract versions, and invalid tensor specs

At the request boundary, Kalandar sends:

- `parser`
- `artifact_id`
- `suite_version`
- `tensor_contract_version`
- `inputs`
- `outputs`

Remote adapter expectations:

- treat `artifact_id` as the bundle identity handshake
- do not mix tokenizer/config/model weights from different bundles
- route the request to model weights that match `parser`, `artifact_id`, `suite_version`, and `tensor_contract_version`
- return tensors using the exact requested names, dtypes, and ranks

`artifact_id` is the content hash for the artifact bundle. The runtime owns tokenization, normalization-version validation, and postprocess-version validation on the client side; the remote side should treat the request metadata as a compatibility check, not as a prompt to re-tokenize.

## Minimal remote tensor boundary example

A user-owned remote adapter must accept a request shaped like this. The `parser`, `artifact_id`, `suite_version`, and `tensor_contract_version` fields are part of the adapter handshake, so a remote service can reject mismatched requests before running inference:

```json
{
  "parser": "task",
  "artifact_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "suite_version": "suite-2026-04-19",
  "tensor_contract_version": "task-bert-token-classification-v1",
  "inputs": {
    "input_ids": {
      "dtype": "int64",
      "shape": [1, 5],
      "data": [101, 7592, 3189, 4820, 102]
    },
    "attention_mask": {
      "dtype": "int64",
      "shape": [1, 5],
      "data": [1, 1, 1, 1, 1]
    }
  },
  "outputs": ["logits"]
}
```

And return:

```ts
{
  outputs: {
    logits: {
      dtype: 'float32',
      shape: [1, 5, 27],
      data: [
        /* 1 * 5 * 27 floats in row-major [batch, token, label] order */
      ],
    },
  },
}
```

The numeric values are model-specific. The shape contract is not optional.

## Minimal adapter sketch

This is the intended client/runtime-side integration shape:

```ts
import { createRemoteInferenceAdapter } from 'kalandar/runtime';

const inference = createRemoteInferenceAdapter({
  kind: 'my-remote-http',
  async run(request) {
    const response = await fetch('https://ml.example.com/kalandar/infer', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Remote inference failed: ${response.status}`);
    }

    return (await response.json());
  },
});
```

The remote service behind that endpoint is user-owned. Kalandar only requires that it accept `TensorRequest` and return `TensorResponse`.

## Artifact hosting note

Artifact loading and artifact publishing are separate concerns.

Kalandar runtime loading can work with artifacts hosted on:

- Hugging Face
- GitHub Releases
- user-hosted mirrors or internal URLs
- local directories

This document freezes the runtime loading boundary only. Publishing those artifacts is a separate release/distribution concern.
