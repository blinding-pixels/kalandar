import type { InferenceAdapter, LoadedMlArtifact, RuntimeTensor } from '../types.ts';

interface OrtTensorLike {
  data: ArrayLike<number> | ArrayLike<bigint>;
  dims: readonly number[];
  type: string;
}

interface OrtSessionLike {
  run(feeds: Record<string, unknown>, outputNames?: string[]): Promise<Record<string, OrtTensorLike>>;
  release?(): Promise<void> | void;
}

interface OrtModuleLike {
  Tensor: new (type: string, data: Int32Array | Float32Array | BigInt64Array, dims: readonly number[]) => unknown;
  InferenceSession: {
    create(model: Uint8Array | string, options?: unknown): Promise<OrtSessionLike>;
  };
}

export interface LocalOnnxAdapterOptions {
  ort?: OrtModuleLike;
  sessionOptions?: unknown;
}

const toOrtTensorData = (tensor: RuntimeTensor): Int32Array | Float32Array | BigInt64Array => {
  if (tensor.dtype === 'float32') {
    return Float32Array.from(tensor.data);
  }
  return BigInt64Array.from(tensor.data.map((value) => BigInt(value)));
};

const fromOrtTensor = (tensor: OrtTensorLike): RuntimeTensor => {
  const data =
    tensor.type === 'int64'
      ? Array.from(tensor.data as ArrayLike<bigint>, (value) => Number(value))
      : Array.from(tensor.data as ArrayLike<number>, (value) => Number(value));

  return {
    dtype: tensor.type === 'int64' ? 'int64' : 'float32',
    shape: Array.from(tensor.dims, (value) => Number(value)),
    data,
  };
};

const resolveOrt = async (provided?: OrtModuleLike): Promise<OrtModuleLike> => {
  if (provided) {
    return provided;
  }

  try {
    const dynamicImport = new Function('specifier', 'return import(specifier)') as (specifier: string) => Promise<unknown>;
    const imported = (await dynamicImport('onnxruntime-node')) as OrtModuleLike;
    return imported;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`onnxruntime-node is required for local ONNX inference: ${message}`);
  }
};

export const createLocalOnnxInferenceAdapter = (options: LocalOnnxAdapterOptions = {}): InferenceAdapter => {
  let session: OrtSessionLike | null = null;
  let ortModule: OrtModuleLike | null = null;

  return {
    kind: 'local-onnx',
    async init(artifact: LoadedMlArtifact): Promise<void> {
      ortModule = await resolveOrt(options.ort);
      session = await ortModule.InferenceSession.create(artifact.files.model.bytes, options.sessionOptions);
    },
    async run(request) {
      if (!session || !ortModule) {
        throw new Error('Local ONNX adapter has not been initialized');
      }

      const feeds = Object.fromEntries(
        Object.entries(request.inputs).map(([name, tensor]) => [
          name,
          new ortModule.Tensor(tensor.dtype, toOrtTensorData(tensor), tensor.shape),
        ]),
      );

      const outputs = await session.run(feeds, request.outputs);
      return {
        outputs: Object.fromEntries(Object.entries(outputs).map(([name, tensor]) => [name, fromOrtTensor(tensor)])),
      };
    },
    async dispose(): Promise<void> {
      await session?.release?.();
      session = null;
    },
  };
};
