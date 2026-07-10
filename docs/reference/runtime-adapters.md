# Runtime Adapters

Runtime adapters connect parser logic to model inference.

Most apps should use `createCapturePreviewParser`. Use runtime adapters when you own model artifacts and want local or remote tensor inference.

## Adapter Interface

```ts
interface InferenceAdapter {
  readonly kind: string;
  init?(artifact: LoadedMlArtifact): Promise<void> | void;
  run(request: TensorRequest): Promise<TensorResponse>;
  dispose?(): Promise<void> | void;
}
```

## Tensor Request

```ts
interface TensorRequest {
  parser: 'calendar' | 'task' | 'reminder' | 'task_decomposition';
  artifact_id: string;
  suite_version: string;
  tensor_contract_version: string;
  inputs: Record<string, RuntimeTensor>;
  outputs: string[];
}
```

## Tensor Response

```ts
interface TensorResponse {
  outputs: Record<string, RuntimeTensor>;
}
```

## Runtime Tensor

```ts
interface RuntimeTensor {
  dtype: 'float32' | 'int64';
  shape: number[];
  data: number[];
}
```

## Local ONNX Adapter

Import the local ONNX adapter:

```ts
import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx';
```

Install `onnxruntime-node` in the host app:

```sh
npm install onnxruntime-node
```

The adapter dynamically imports `onnxruntime-node` unless you pass an ORT module explicitly.

## Local Core AI Adapter

Import the Core AI adapter:

```ts
import { createLocalCoreAiInferenceAdapter } from 'kalandar/runtime/coreai';
```

The host provides a bridge with three operations:

- `loadModel`
- `runModel`
- `disposeModel`

The JavaScript package does not load Apple-only Core AI libraries directly. Native
Swift apps can bypass this bridge and use `CoreAIParserSuite` or
`CoreAICapturePreviewService` with a Core AI artifact root.

## Remote Adapter

Import the remote adapter:

```ts
import { createRemoteInferenceAdapter } from 'kalandar/runtime';
```

Remote adapters should treat request metadata as a compatibility check. They should not re-tokenize input or reinterpret parser semantics.

## Tensor Contract

The token-classifier tensor contract uses:

- `input_ids`
- `attention_mask`
- `logits`
- no padding at the Kalandar boundary
- tail truncation before inference
- empty input handled before tensor inference

For full details, see [Tensor contract](../../src/runtime/TENSOR_CONTRACT.md).

## What's Next

- Learn the artifact model in [Artifacts and inference](../concepts/artifacts-and-inference.md).
- Self-host inference with [Self-host inference](../tasks/self-host-inference.md).
- Use native models in [Use Swift and Core AI](../tasks/use-coreai.md).
- Review package contents in [Package layout](./package-layout.md).
