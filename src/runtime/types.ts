export type MlParserName = 'calendar' | 'task' | 'reminder' | 'task_decomposition';
export type SupportedMlArtifactParser = MlParserName;

export type RuntimeTensorDtype = 'float32' | 'int64';

export interface RuntimeTensor {
  dtype: RuntimeTensorDtype;
  shape: number[];
  data: number[];
}

export interface TensorSpec {
  name: string;
  dtype: RuntimeTensorDtype;
  rank: number;
}

export interface TensorContract {
  inputs: TensorSpec[];
  outputs: TensorSpec[];
  max_sequence_length: number;
  padding: 'none';
  truncation: 'tail';
  empty_input_behavior: 'return_empty_result';
  whitespace_input_behavior: 'return_empty_result';
  oversized_input_behavior: 'truncate';
}

export interface ArtifactFileSpec {
  path: string;
  sha256: string;
  size_bytes: number;
}

export interface BertWordPieceTokenizerManifest {
  kind: 'bert_wordpiece';
  do_lower_case: boolean;
  unk_token: string;
  cls_token: string;
  sep_token: string;
  pad_token: string;
  max_sequence_length: number;
}

export interface MlArtifactManifest {
  manifest_version: string;
  parser: SupportedMlArtifactParser;
  suite_version: string;
  artifact_id: string;
  tensor_contract_version: string;
  normalization_version: string;
  postprocess_version: string;
  tokenizer: BertWordPieceTokenizerManifest;
  tensor_contract: TensorContract;
  files: {
    model: ArtifactFileSpec;
    config: ArtifactFileSpec;
    tokenizer_config: ArtifactFileSpec;
    special_tokens_map: ArtifactFileSpec;
    vocab: ArtifactFileSpec;
    tokenizer_json?: ArtifactFileSpec;
  };
}

export type CalendarArtifactManifest = MlArtifactManifest & {
  parser: 'calendar';
};

export type TaskArtifactManifest = MlArtifactManifest & {
  parser: 'task';
};

export type ReminderArtifactManifest = MlArtifactManifest & {
  parser: 'reminder';
};

export type TaskDecompositionArtifactManifest = MlArtifactManifest & {
  parser: 'task_decomposition';
};

export interface ArtifactManifestByParser {
  calendar: CalendarArtifactManifest;
  task: TaskArtifactManifest;
  reminder: ReminderArtifactManifest;
  task_decomposition: TaskDecompositionArtifactManifest;
}

export interface TokenizedText {
  inputIds: number[];
  attentionMask: number[];
  offsets: Array<[number, number]>;
  truncated: boolean;
}

export interface RuntimeTokenizer {
  readonly kind: 'bert_wordpiece';
  readonly maxSequenceLength: number;
  readonly specialTokenIds: Set<number>;
  encode(input: string): TokenizedText;
}

export interface LoadedArtifactFile {
  path: string;
  sha256: string;
  sizeBytes: number;
  bytes: Uint8Array;
  text?: string;
  url?: string;
  absolutePath?: string;
}

export interface LoadedArtifactFiles {
  model: LoadedArtifactFile;
  config: LoadedArtifactFile;
  tokenizer_config: LoadedArtifactFile;
  special_tokens_map: LoadedArtifactFile;
  vocab: LoadedArtifactFile;
  tokenizer_json?: LoadedArtifactFile;
}

export interface LoadedMlArtifact<TParser extends SupportedMlArtifactParser = SupportedMlArtifactParser> {
  parser: TParser;
  suite_version: string;
  artifact_id: string;
  manifest: ArtifactManifestByParser[TParser];
  tokenizer: RuntimeTokenizer;
  files: LoadedArtifactFiles;
  idToLabel: Record<number, string>;
  labelToId: Record<string, number>;
}

export type MlArtifactSource =
  | { kind: 'directory'; path: string }
  | { kind: 'url'; url: string }
  | { kind: 'huggingface'; repo: string; parser: SupportedMlArtifactParser; revision?: string }
  | { kind: 'manifest'; url: string };

export interface TensorRequest {
  parser: SupportedMlArtifactParser;
  artifact_id: string;
  suite_version: string;
  tensor_contract_version: string;
  inputs: Record<string, RuntimeTensor>;
  outputs: string[];
}

export interface TensorResponse {
  outputs: Record<string, RuntimeTensor>;
}

export interface InferenceAdapter {
  readonly kind: string;
  init?(artifact: LoadedMlArtifact): Promise<void> | void;
  run(request: TensorRequest): Promise<TensorResponse>;
  dispose?(): Promise<void> | void;
}

export interface RemoteInferenceAdapterOptions {
  kind?: string;
  init?(artifact: LoadedMlArtifact): Promise<void> | void;
  run(request: TensorRequest): Promise<TensorResponse>;
  dispose?(): Promise<void> | void;
}
