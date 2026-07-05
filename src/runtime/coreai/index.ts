import type { InferenceAdapter, LoadedMlArtifact, RuntimeTensor, TensorRequest, TensorResponse } from '../types.ts';

export type CoreAiModelHandle = unknown;

export interface CoreAiLoadModelOptions {
  artifact: LoadedMlArtifact;
  modelPath?: string;
  modelBytes: Uint8Array;
}

export interface CoreAiRuntimeBridge {
  loadModel(options: CoreAiLoadModelOptions): Promise<CoreAiModelHandle> | CoreAiModelHandle;
  runModel(
    model: CoreAiModelHandle,
    request: TensorRequest,
  ): Promise<TensorResponse | Record<string, RuntimeTensor>> | TensorResponse | Record<string, RuntimeTensor>;
  disposeModel?(model: CoreAiModelHandle): Promise<void> | void;
}

export interface LocalCoreAiAdapterOptions {
  bridge: CoreAiRuntimeBridge;
  kind?: string;
  modelPath?: string;
  requireModelPath?: boolean;
}

const normalizeResponse = (response: TensorResponse | Record<string, RuntimeTensor>): TensorResponse => {
  if ('outputs' in response && response.outputs && typeof response.outputs === 'object') {
    return response as TensorResponse;
  }
  return { outputs: response as Record<string, RuntimeTensor> };
};

const resolveModelPath = (artifact: LoadedMlArtifact, options: LocalCoreAiAdapterOptions): string | undefined =>
  options.modelPath || artifact.files.model.absolutePath;

export const createLocalCoreAiInferenceAdapter = (options: LocalCoreAiAdapterOptions): InferenceAdapter => {
  let model: CoreAiModelHandle;
  let initialized = false;

  return {
    kind: options.kind || 'local-coreai',
    async init(artifact: LoadedMlArtifact): Promise<void> {
      const modelPath = resolveModelPath(artifact, options);
      if ((options.requireModelPath ?? true) && !modelPath) {
        throw new Error('Local Core AI adapter requires a local model path; load the artifact from a directory or pass modelPath');
      }

      model = await options.bridge.loadModel({
        artifact,
        modelPath,
        modelBytes: artifact.files.model.bytes,
      });
      initialized = true;
    },
    async run(request: TensorRequest): Promise<TensorResponse> {
      if (!initialized) {
        throw new Error('Local Core AI adapter has not been initialized');
      }

      return normalizeResponse(await options.bridge.runModel(model, request));
    },
    async dispose(): Promise<void> {
      if (initialized) {
        await options.bridge.disposeModel?.(model);
      }
      model = undefined;
      initialized = false;
    },
  };
};
