/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createLocalOnnxInferenceAdapter } from '../onnx/index.ts';
import { createCalendarTestBundle, createReminderTestBundle, createTaskTestBundle } from './helpers.ts';
import { loadMlArtifact } from '../index.ts';

describe('local ONNX adapter', () => {
  test('converts tensors at the runtime boundary', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir });
      const calls: Array<{ modelLength: number; feeds?: Record<string, unknown>; outputs?: string[] }> = [];

      const adapter = createLocalOnnxInferenceAdapter({
        ort: {
          Tensor: class FakeTensor {
            type: string;
            data: Int32Array | Float32Array | BigInt64Array;
            dims: readonly number[];

            constructor(type: string, data: Int32Array | Float32Array | BigInt64Array, dims: readonly number[]) {
              this.type = type;
              this.data = data;
              this.dims = dims;
            }
          },
          InferenceSession: {
            async create(model: Uint8Array) {
              calls.push({ modelLength: model.byteLength });
              return {
                async run(feeds: Record<string, unknown>, outputs?: string[]) {
                  calls[calls.length - 1].feeds = feeds;
                  calls[calls.length - 1].outputs = outputs;
                  return {
                    logits: {
                      type: 'float32',
                      dims: [1, 3, 2],
                      data: new Float32Array([0, 1, 1, 0, 0, 1]),
                    },
                  };
                },
              };
            },
          },
        },
      });

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

      assert.equal(calls[0].modelLength, artifact.files.model.bytes.byteLength);
      assert.deepEqual(calls[0].outputs, ['logits']);
      assert.ok(calls[0].feeds?.input_ids);
      assert.deepEqual(response.outputs.logits.shape, [1, 3, 2]);
      assert.equal(response.outputs.logits.dtype, 'float32');
      assert.deepEqual(response.outputs.logits.data, [0, 1, 1, 0, 0, 1]);
    } finally {
      await bundle.cleanup();
    }
  });

  test('fails loudly when run before init', async () => {
    const adapter = createLocalOnnxInferenceAdapter({
      ort: {
        Tensor: class FakeTensor {
          type: string;
          data: Int32Array | Float32Array | BigInt64Array;
          dims: readonly number[];

          constructor(type: string, data: Int32Array | Float32Array | BigInt64Array, dims: readonly number[]) {
            this.type = type;
            this.data = data;
            this.dims = dims;
          }
        },
        InferenceSession: {
          async create() {
            throw new Error('should not be called');
          },
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

  test('accepts task artifacts through the same tensor boundary', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task');
      const adapter = createLocalOnnxInferenceAdapter({
        ort: {
          Tensor: class FakeTensor {
            type: string;
            data: Int32Array | Float32Array | BigInt64Array;
            dims: readonly number[];

            constructor(type: string, data: Int32Array | Float32Array | BigInt64Array, dims: readonly number[]) {
              this.type = type;
              this.data = data;
              this.dims = dims;
            }
          },
          InferenceSession: {
            async create() {
              return {
                async run() {
                  return {
                    logits: {
                      type: 'float32',
                      dims: [1, 4, 3],
                      data: new Float32Array([0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0]),
                    },
                  };
                },
              };
            },
          },
        },
      });

      await adapter.init?.(artifact);
      const response = await adapter.run({
        parser: 'task',
        artifact_id: artifact.artifact_id,
        suite_version: artifact.suite_version,
        tensor_contract_version: artifact.manifest.tensor_contract_version,
        inputs: {
          input_ids: { dtype: 'int64', shape: [1, 4], data: [101, 102, 103, 104] },
          attention_mask: { dtype: 'int64', shape: [1, 4], data: [1, 1, 1, 1] },
        },
        outputs: ['logits'],
      });

      assert.deepEqual(response.outputs.logits.shape, [1, 4, 3]);
      assert.equal(response.outputs.logits.dtype, 'float32');
    } finally {
      await bundle.cleanup();
    }
  });

  test('accepts reminder artifacts through the same tensor boundary', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder');
      const adapter = createLocalOnnxInferenceAdapter({
        ort: {
          Tensor: class FakeTensor {
            type: string;
            data: Int32Array | Float32Array | BigInt64Array;
            dims: readonly number[];

            constructor(type: string, data: Int32Array | Float32Array | BigInt64Array, dims: readonly number[]) {
              this.type = type;
              this.data = data;
              this.dims = dims;
            }
          },
          InferenceSession: {
            async create() {
              return {
                async run() {
                  return {
                    logits: {
                      type: 'float32',
                      dims: [1, 5, 4],
                      data: new Float32Array([0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0]),
                    },
                  };
                },
              };
            },
          },
        },
      });

      await adapter.init?.(artifact);
      const response = await adapter.run({
        parser: 'reminder',
        artifact_id: artifact.artifact_id,
        suite_version: artifact.suite_version,
        tensor_contract_version: artifact.manifest.tensor_contract_version,
        inputs: {
          input_ids: { dtype: 'int64', shape: [1, 5], data: [101, 102, 103, 104, 105] },
          attention_mask: { dtype: 'int64', shape: [1, 5], data: [1, 1, 1, 1, 1] },
        },
        outputs: ['logits'],
      });

      assert.deepEqual(response.outputs.logits.shape, [1, 5, 4]);
      assert.equal(response.outputs.logits.dtype, 'float32');
    } finally {
      await bundle.cleanup();
    }
  });
});
