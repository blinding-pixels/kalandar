/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createLocalCoreAiInferenceAdapter } from '../coreai/index.ts';
import { createCalendarTestBundle } from './helpers.ts';
import { loadMlArtifact } from '../index.ts';
import type { CoreAiLoadModelOptions, CoreAiModelHandle } from '../coreai/index.ts';
import type { TensorRequest } from '../types.ts';

describe('local Core AI adapter', () => {
  test('delegates model load and tensor inference to the host bridge', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir });
      const calls: Array<{ load?: CoreAiLoadModelOptions; request?: TensorRequest; disposed?: CoreAiModelHandle }> = [];
      const handle = { id: 'coreai-model' };

      const adapter = createLocalCoreAiInferenceAdapter({
        bridge: {
          loadModel(options) {
            calls.push({ load: options });
            return handle;
          },
          runModel(model, request) {
            assert.equal(model, handle);
            calls[calls.length - 1].request = request;
            return {
              logits: {
                dtype: 'float32',
                shape: [1, 3, 2],
                data: [0, 1, 1, 0, 0, 1],
              },
            };
          },
          disposeModel(model) {
            calls[calls.length - 1].disposed = model;
          },
        },
      });

      assert.equal(adapter.kind, 'local-coreai');
      await adapter.init?.(artifact);
      const response = await adapter.run({
        parser: 'calendar',
        artifact_id: artifact.artifact_id,
        suite_version: artifact.suite_version,
        tensor_contract_version: artifact.manifest.tensor_contract_version,
        inputs: {
          input_ids: { dtype: 'int64', shape: [1, 3], data: [101, 102, 103] },
          attention_mask: { dtype: 'int64', shape: [1, 3], data: [1, 1, 1] },
        },
        outputs: ['logits'],
      });
      await adapter.dispose?.();

      assert.equal(calls[0].load?.artifact, artifact);
      assert.equal(calls[0].load?.modelPath, artifact.files.model.absolutePath);
      assert.equal(calls[0].load?.modelBytes.byteLength, artifact.files.model.bytes.byteLength);
      assert.deepEqual(calls[0].request?.outputs, ['logits']);
      assert.deepEqual(response.outputs.logits.shape, [1, 3, 2]);
      assert.equal(response.outputs.logits.dtype, 'float32');
      assert.deepEqual(response.outputs.logits.data, [0, 1, 1, 0, 0, 1]);
      assert.equal(calls[0].disposed, handle);
    } finally {
      await bundle.cleanup();
    }
  });

  test('fails loudly when run before init', async () => {
    const adapter = createLocalCoreAiInferenceAdapter({
      bridge: {
        loadModel() {
          throw new Error('should not be called');
        },
        runModel() {
          throw new Error('should not be called');
        },
      },
    });

    await assert.rejects(
      () =>
        adapter.run({
          parser: 'calendar',
          artifact_id: 'sha256:abc',
          suite_version: 'suite',
          tensor_contract_version: 'v1',
          inputs: {},
          outputs: ['logits'],
        }),
      /not been initialized/i,
    );
  });

  test('requires a local model path by default', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir });
      const artifactWithoutPath = {
        ...artifact,
        files: {
          ...artifact.files,
          model: {
            ...artifact.files.model,
            absolutePath: undefined,
          },
        },
      };
      const adapter = createLocalCoreAiInferenceAdapter({
        bridge: {
          loadModel() {
            throw new Error('should not be called');
          },
          runModel() {
            throw new Error('should not be called');
          },
        },
      });

      await assert.rejects(async () => {
        await adapter.init?.(artifactWithoutPath);
      }, /requires a local model path/i);
    } finally {
      await bundle.cleanup();
    }
  });
});
