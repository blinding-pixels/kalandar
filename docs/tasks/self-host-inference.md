# Self-Host Inference

This task shows how to connect Kalandar to model inference that you host.

## Before You Begin

You need:

- A valid Kalandar artifact bundle.
- A host that can run the model locally or remotely.
- An adapter that implements `InferenceAdapter`.

The base npm package does not include model artifacts.

## Choose An Adapter

Use one of these adapter shapes:

| Adapter | Use when |
| --- | --- |
| `createLocalOnnxInferenceAdapter` | Node runs ONNX locally with `onnxruntime-node`. |
| `createLocalCoreAiInferenceAdapter` | An Apple host loads Core AI models through a native bridge. |
| `createRemoteInferenceAdapter` | You run inference on user-owned remote infrastructure. |

## Load An Artifact

Load an artifact from a directory, URL, manifest URL, or Hugging Face source:

```ts
import { loadMlArtifact } from 'kalandar/runtime';

const artifact = await loadMlArtifact({
  kind: 'directory',
  path: '/path/to/task-artifact',
});
```

The runtime validates the manifest, file digests, tensor contract, labels, normalization version, and postprocess version.

## Use Local ONNX

Install ONNX Runtime:

```sh
npm install onnxruntime-node
```

Create and initialize the adapter:

```ts
import { createLocalOnnxInferenceAdapter } from 'kalandar/runtime/onnx';

const adapter = createLocalOnnxInferenceAdapter();

await adapter.init?.(artifact);
```

## Use Remote Inference

Create an adapter that sends tensor requests to your service:

```ts
import { createRemoteInferenceAdapter } from 'kalandar/runtime';

const adapter = createRemoteInferenceAdapter({
  kind: 'my-inference-service',
  async run(request) {
    const response = await fetch('https://inference.example.com/kalandar', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Inference failed: ${response.status}`);
    }

    return await response.json();
  },
});
```

The remote service receives tensors, not raw app objects. It must return tensors that match the requested output names and shapes.

## Dispose Resources

Dispose an adapter when your app no longer needs it:

```ts
await adapter.dispose?.();
```

## What's Next

- Review tensor shapes in [Runtime adapters](../reference/runtime-adapters.md).
- Read the detailed tensor contract in [Tensor contract](../../src/runtime/TENSOR_CONTRACT.md).
- Learn the artifact model in [Artifacts and inference](../concepts/artifacts-and-inference.md).
