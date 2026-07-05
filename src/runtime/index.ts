import type {
  InferenceAdapter,
  LoadedMlArtifact,
  RemoteInferenceAdapterOptions,
  TensorRequest,
  TensorResponse,
} from './types.ts';

export type {
  ArtifactFileSpec,
  CalendarArtifactManifest,
  InferenceAdapter,
  LoadedArtifactFile,
  LoadedArtifactFiles,
  LoadedMlArtifact,
  MlArtifactManifest,
  MlArtifactSource,
  MlParserName,
  ReminderArtifactManifest,
  RemoteInferenceAdapterOptions,
  RuntimeTensor,
  RuntimeTensorDtype,
  SupportedMlArtifactParser,
  TaskArtifactManifest,
  TaskDecompositionArtifactManifest,
  RuntimeTokenizer,
  TensorContract,
  TensorRequest,
  TensorResponse,
  TensorSpec,
  TokenizedText,
} from './types.ts';
export type { CoreAiLoadModelOptions, CoreAiModelHandle, CoreAiRuntimeBridge, LocalCoreAiAdapterOptions } from './coreai/index.ts';
export {
  ARTIFACT_MANIFEST_VERSION,
  CALENDAR_NORMALIZATION_VERSION,
  CALENDAR_POSTPROCESS_VERSION,
  CALENDAR_TENSOR_CONTRACT_VERSION,
  REMINDER_NORMALIZATION_VERSION,
  REMINDER_POSTPROCESS_VERSION,
  REMINDER_TENSOR_CONTRACT_VERSION,
  TASK_DECOMPOSITION_NORMALIZATION_VERSION,
  TASK_DECOMPOSITION_POSTPROCESS_VERSION,
  TASK_DECOMPOSITION_TENSOR_CONTRACT_VERSION,
  TASK_NORMALIZATION_VERSION,
  TASK_POSTPROCESS_VERSION,
  TASK_TENSOR_CONTRACT_VERSION,
  artifactFromHuggingFace,
  computeArtifactId,
  loadMlArtifact,
} from './artifacts.ts';
export { createLocalCoreAiInferenceAdapter } from './coreai/index.ts';

export const createRemoteInferenceAdapter = (options: RemoteInferenceAdapterOptions): InferenceAdapter => ({
  kind: options.kind || 'remote',
  init: (artifact: LoadedMlArtifact) => options.init?.(artifact),
  run: (request: TensorRequest): Promise<TensorResponse> => options.run(request),
  dispose: () => options.dispose?.(),
});
