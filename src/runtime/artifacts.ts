import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';

import { isSequenceDecoding, orderedLabels, validateBioLabels, validateSequenceDecoderSidecar } from './sequence-decoder.ts';
import { createBertWordPieceTokenizer } from './tokenizer.ts';
import type {
  ArtifactManifestByParser,
  LoadedArtifactFile,
  LoadedArtifactFiles,
  LoadedMlArtifact,
  MlArtifactManifest,
  MlArtifactSource,
  SupportedMlArtifactParser,
} from './types.ts';

const textDecoder = new TextDecoder();

export const ARTIFACT_MANIFEST_VERSION = 'kalandar-ml-artifact-v1';
export const CALENDAR_TENSOR_CONTRACT_VERSION = 'calendar-bert-token-classification-v1';
export const CALENDAR_NORMALIZATION_VERSION = 'calendar-bert-uncased-v1';
export const CALENDAR_POSTPROCESS_VERSION = 'calendar-event-postprocess-v2';
export const TASK_TENSOR_CONTRACT_VERSION = 'task-bert-token-classification-v1';
export const TASK_NORMALIZATION_VERSION = 'task-bert-uncased-v1';
export const TASK_POSTPROCESS_VERSION = 'task-postprocess-v2';
export const TASK_DECOMPOSITION_TENSOR_CONTRACT_VERSION = 'task-decomposition-bert-token-classification-v1';
export const TASK_DECOMPOSITION_NORMALIZATION_VERSION = 'task-decomposition-bert-uncased-v1';
export const TASK_DECOMPOSITION_POSTPROCESS_VERSION = 'task-decomposition-postprocess-v1';
export const REMINDER_TENSOR_CONTRACT_VERSION = 'reminder-bert-token-classification-v1';
export const REMINDER_NORMALIZATION_VERSION = 'reminder-bert-uncased-v1';
export const REMINDER_POSTPROCESS_VERSION = 'reminder-postprocess-v2';

const CALENDAR_REQUIRED_LABELS = [
  'O',
  'B-title',
  'I-title',
  'B-start_time',
  'I-start_time',
  'B-end_time',
  'I-end_time',
  'B-recurrence_start',
  'I-recurrence_start',
  'B-recurrence_end',
  'I-recurrence_end',
  'B-recurrence_exception',
  'I-recurrence_exception',
  'B-duration',
  'I-duration',
  'B-location',
  'I-location',
  'B-recurrence',
  'I-recurrence',
].sort();

const TASK_REQUIRED_LABELS = [
  'O',
  'B-title',
  'I-title',
  'B-due_date',
  'I-due_date',
  'B-due_time',
  'I-due_time',
  'B-priority',
  'I-priority',
  'B-assignee',
  'I-assignee',
  'B-project',
  'I-project',
  'B-tags',
  'I-tags',
  'B-duration',
  'I-duration',
  'B-status',
  'I-status',
  'B-recurrence',
  'I-recurrence',
  'B-recurrence_start',
  'I-recurrence_start',
  'B-recurrence_end',
  'I-recurrence_end',
  'B-recurrence_exception',
  'I-recurrence_exception',
].sort();

const REMINDER_REQUIRED_LABELS = [
  'O',
  'B-title',
  'I-title',
  'B-trigger_time',
  'I-trigger_time',
  'B-recurrence',
  'I-recurrence',
  'B-recurrence_start',
  'I-recurrence_start',
  'B-recurrence_end',
  'I-recurrence_end',
  'B-recurrence_exception',
  'I-recurrence_exception',
  'B-recurrence_window',
  'I-recurrence_window',
  'B-location',
  'I-location',
  'B-lead_time',
  'I-lead_time',
].sort();

const TASK_DECOMPOSITION_REQUIRED_LABELS = [
  'O',
  'B-parent_title',
  'I-parent_title',
  'B-child_item',
  'I-child_item',
  'B-child_target',
  'I-child_target',
  'B-axis_subject',
  'I-axis_subject',
  'B-axis_action',
  'I-axis_action',
  'B-shared_due_date',
  'I-shared_due_date',
  'B-shared_due_time',
  'I-shared_due_time',
  'B-child_due_date',
  'I-child_due_date',
  'B-child_due_time',
  'I-child_due_time',
  'B-shared_project',
  'I-shared_project',
].sort();

const SUPPORTED_ARTIFACT_PARSERS = new Set<SupportedMlArtifactParser>(['calendar', 'task', 'reminder', 'task_decomposition']);
const artifactIdPattern = /^sha256:[0-9a-f]{64}$/;

interface ParserRuntimeSpec {
  tensorContractVersion: string;
  normalizationVersion: string;
  postprocessVersion: string;
  requiredLabels: string[];
}

const PARSER_RUNTIME_SPECS: Record<SupportedMlArtifactParser, ParserRuntimeSpec> = {
  calendar: {
    tensorContractVersion: CALENDAR_TENSOR_CONTRACT_VERSION,
    normalizationVersion: CALENDAR_NORMALIZATION_VERSION,
    postprocessVersion: CALENDAR_POSTPROCESS_VERSION,
    requiredLabels: CALENDAR_REQUIRED_LABELS,
  },
  task: {
    tensorContractVersion: TASK_TENSOR_CONTRACT_VERSION,
    normalizationVersion: TASK_NORMALIZATION_VERSION,
    postprocessVersion: TASK_POSTPROCESS_VERSION,
    requiredLabels: TASK_REQUIRED_LABELS,
  },
  reminder: {
    tensorContractVersion: REMINDER_TENSOR_CONTRACT_VERSION,
    normalizationVersion: REMINDER_NORMALIZATION_VERSION,
    postprocessVersion: REMINDER_POSTPROCESS_VERSION,
    requiredLabels: REMINDER_REQUIRED_LABELS,
  },
  task_decomposition: {
    tensorContractVersion: TASK_DECOMPOSITION_TENSOR_CONTRACT_VERSION,
    normalizationVersion: TASK_DECOMPOSITION_NORMALIZATION_VERSION,
    postprocessVersion: TASK_DECOMPOSITION_POSTPROCESS_VERSION,
    requiredLabels: TASK_DECOMPOSITION_REQUIRED_LABELS,
  },
};

const isObject = (value: unknown): value is Record<string, unknown> => Boolean(value) && typeof value === 'object';

const stableValue = (value: unknown): unknown => {
  if (Array.isArray(value)) {
    return value.map((entry) => stableValue(entry));
  }
  if (!isObject(value)) {
    return value;
  }
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, stableValue(value[key])]),
  );
};

export const computeArtifactId = (manifest: Omit<MlArtifactManifest, 'artifact_id'>): string => {
  const normalized = JSON.stringify(stableValue(manifest));
  const hash = createHash('sha256').update(normalized).digest('hex');
  return `sha256:${hash}`;
};

const sha256Hex = (bytes: Uint8Array): string => createHash('sha256').update(bytes).digest('hex');

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function assertSupportedArtifactParser(parser: unknown): asserts parser is SupportedMlArtifactParser {
  assert(typeof parser === 'string' && SUPPORTED_ARTIFACT_PARSERS.has(parser as SupportedMlArtifactParser), `Unsupported artifact parser ${String(parser)}`);
}

const toManifestUrl = (source: MlArtifactSource): string => {
  if (source.kind === 'manifest') {
    return source.url;
  }
  if (source.kind === 'url') {
    return source.url.endsWith('.json') ? source.url : `${source.url.replace(/\/+$/, '')}/manifest.json`;
  }
  if (source.kind === 'huggingface') {
    const revision = source.revision || 'main';
    return `https://huggingface.co/${source.repo}/resolve/${revision}/${source.parser}/manifest.json`;
  }
  throw new Error(`Source kind ${source.kind} does not use a manifest URL`);
};

export const artifactFromHuggingFace = (source: {
  repo: string;
  parser: SupportedMlArtifactParser;
  revision?: string;
}): MlArtifactSource => ({
  kind: 'huggingface',
  repo: source.repo,
  parser: source.parser,
  revision: source.revision,
});

const parseManifest = (payload: string): MlArtifactManifest => {
  const manifest = JSON.parse(payload) as MlArtifactManifest;
  assert(isObject(manifest), 'Artifact manifest must be an object');
  assert(manifest.manifest_version === ARTIFACT_MANIFEST_VERSION, 'Unsupported artifact manifest version');
  assertSupportedArtifactParser(manifest.parser);
  assert(typeof manifest.suite_version === 'string' && manifest.suite_version.trim().length > 0, 'Artifact manifest is missing suite_version');
  assert(typeof manifest.artifact_id === 'string' && artifactIdPattern.test(manifest.artifact_id), 'Artifact manifest has an invalid artifact_id');
  assert(typeof manifest.tensor_contract_version === 'string', 'Artifact manifest is missing tensor_contract_version');
  assert(typeof manifest.normalization_version === 'string', 'Artifact manifest is missing normalization_version');
  assert(typeof manifest.postprocess_version === 'string', 'Artifact manifest is missing postprocess_version');
  assert(manifest.tokenizer?.kind === 'bert_wordpiece', 'Artifact manifest tokenizer kind is unsupported');
  assert(manifest.tensor_contract?.inputs?.length > 0, 'Artifact manifest is missing tensor input definitions');
  assert(manifest.tensor_contract?.outputs?.length > 0, 'Artifact manifest is missing tensor output definitions');
  assert(manifest.files?.model, 'Artifact manifest is missing model file metadata');
  assert(manifest.files?.config, 'Artifact manifest is missing config file metadata');
  assert(manifest.files?.tokenizer_config, 'Artifact manifest is missing tokenizer_config file metadata');
  assert(manifest.files?.special_tokens_map, 'Artifact manifest is missing special_tokens_map file metadata');
  assert(manifest.files?.vocab, 'Artifact manifest is missing vocab file metadata');
  assert(
    manifest.decoding === undefined || isSequenceDecoding(manifest.decoding),
    `Artifact manifest decoding must be "argmax" or "bio", received ${JSON.stringify(manifest.decoding)}`,
  );
  return manifest;
};

const fetchBytes = async (url: string): Promise<Uint8Array> => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load artifact file from ${url}: ${response.status} ${response.statusText}`);
  }
  const buffer = await response.arrayBuffer();
  return new Uint8Array(buffer);
};

const loadDirectoryFile = async (
  root: string,
  filePath: string,
  expectedSha: string,
  expectedSize: number,
): Promise<LoadedArtifactFile> => {
  const absolutePath = path.resolve(root, filePath);
  const normalizedRoot = `${path.resolve(root)}${path.sep}`;
  assert(
    absolutePath === path.resolve(root) || absolutePath.startsWith(normalizedRoot),
    `Artifact file ${filePath} escapes the bundle root`,
  );
  const bytes = new Uint8Array(await fs.readFile(absolutePath));
  const actualSha = sha256Hex(bytes);
  assert(actualSha === expectedSha, `Checksum mismatch for ${filePath}`);
  assert(bytes.byteLength === expectedSize, `Size mismatch for ${filePath}`);
  return {
    path: filePath,
    sha256: actualSha,
    sizeBytes: bytes.byteLength,
    bytes,
    absolutePath,
  };
};

const loadRemoteFile = async (
  baseUrl: string,
  filePath: string,
  expectedSha: string,
  expectedSize: number,
): Promise<LoadedArtifactFile> => {
  assert(!filePath.split('/').includes('..'), `Artifact file ${filePath} escapes the remote bundle root`);
  const url = new URL(filePath, baseUrl).toString();
  const bytes = await fetchBytes(url);
  const actualSha = sha256Hex(bytes);
  assert(actualSha === expectedSha, `Checksum mismatch for ${url}`);
  assert(bytes.byteLength === expectedSize, `Size mismatch for ${url}`);
  return {
    path: filePath,
    sha256: actualSha,
    sizeBytes: bytes.byteLength,
    bytes,
    url,
  };
};

const loadArtifactFiles = async (
  source: MlArtifactSource,
  manifestUrlOrRoot: string,
  manifest: MlArtifactManifest,
): Promise<LoadedArtifactFiles> => {
  const loadFile =
    source.kind === 'directory'
      ? (filePath: string, sha: string, size: number) => loadDirectoryFile(manifestUrlOrRoot, filePath, sha, size)
      : (filePath: string, sha: string, size: number) => loadRemoteFile(manifestUrlOrRoot, filePath, sha, size);

  const files: LoadedArtifactFiles = {
    model: await loadFile(manifest.files.model.path, manifest.files.model.sha256, manifest.files.model.size_bytes),
    config: await loadFile(manifest.files.config.path, manifest.files.config.sha256, manifest.files.config.size_bytes),
    tokenizer_config: await loadFile(
      manifest.files.tokenizer_config.path,
      manifest.files.tokenizer_config.sha256,
      manifest.files.tokenizer_config.size_bytes,
    ),
    special_tokens_map: await loadFile(
      manifest.files.special_tokens_map.path,
      manifest.files.special_tokens_map.sha256,
      manifest.files.special_tokens_map.size_bytes,
    ),
    vocab: await loadFile(manifest.files.vocab.path, manifest.files.vocab.sha256, manifest.files.vocab.size_bytes),
  };

  if (manifest.files.tokenizer_json) {
    files.tokenizer_json = await loadFile(
      manifest.files.tokenizer_json.path,
      manifest.files.tokenizer_json.sha256,
      manifest.files.tokenizer_json.size_bytes,
    );
  }

  files.config.text = textDecoder.decode(files.config.bytes);
  files.tokenizer_config.text = textDecoder.decode(files.tokenizer_config.bytes);
  files.special_tokens_map.text = textDecoder.decode(files.special_tokens_map.bytes);
  files.vocab.text = textDecoder.decode(files.vocab.bytes);
  if (files.tokenizer_json) {
    files.tokenizer_json.text = textDecoder.decode(files.tokenizer_json.bytes);
  }
  if (manifest.files.sequence_decoder) {
    files.sequence_decoder = await loadFile(
      manifest.files.sequence_decoder.path,
      manifest.files.sequence_decoder.sha256,
      manifest.files.sequence_decoder.size_bytes,
    );
    files.sequence_decoder.text = textDecoder.decode(files.sequence_decoder.bytes);
  }

  return files;
};

const validateTensorContract = (parser: SupportedMlArtifactParser, manifest: MlArtifactManifest): void => {
  assert(
    Number.isInteger(manifest.tensor_contract.max_sequence_length) &&
      manifest.tensor_contract.max_sequence_length >= 2 &&
      manifest.tensor_contract.max_sequence_length <= 512,
    `${parser} artifact must declare max_sequence_length from 2 through 512`,
  );
  assert(
    manifest.tokenizer.max_sequence_length === manifest.tensor_contract.max_sequence_length,
    `${parser} artifact tokenizer and tensor contract max_sequence_length mismatch`,
  );
  assert(manifest.tensor_contract.padding === 'none', `${parser} artifact padding policy must be none`);
  assert(manifest.tensor_contract.truncation === 'tail', `${parser} artifact truncation policy must be tail`);
  assert(manifest.tensor_contract.inputs.length === 2, `${parser} artifact must declare two tensor inputs`);
  assert(manifest.tensor_contract.outputs.length === 1, `${parser} artifact must declare one tensor output`);
  assert(
    manifest.tensor_contract.inputs[0].name === 'input_ids' &&
      manifest.tensor_contract.inputs[0].dtype === 'int64' &&
      manifest.tensor_contract.inputs[0].rank === 2,
    `${parser} artifact input_ids tensor contract mismatch`,
  );
  assert(
    manifest.tensor_contract.inputs[1].name === 'attention_mask' &&
      manifest.tensor_contract.inputs[1].dtype === 'int64' &&
      manifest.tensor_contract.inputs[1].rank === 2,
    `${parser} artifact attention_mask tensor contract mismatch`,
  );
  assert(
    manifest.tensor_contract.outputs[0].name === 'logits' &&
      manifest.tensor_contract.outputs[0].dtype === 'float32' &&
      manifest.tensor_contract.outputs[0].rank === 3,
    `${parser} artifact logits tensor contract mismatch`,
  );
};

function validateManifestForParser<TParser extends SupportedMlArtifactParser>(
  manifest: MlArtifactManifest,
  parser: TParser,
): asserts manifest is ArtifactManifestByParser[TParser] {
  const spec = PARSER_RUNTIME_SPECS[parser];
  assert(manifest.parser === parser, `Artifact parser mismatch: expected ${parser}, received ${String(manifest.parser)}`);
  assert(manifest.tensor_contract_version === spec.tensorContractVersion, `Unsupported ${parser} tensor_contract_version`);
  assert(manifest.normalization_version === spec.normalizationVersion, `Unsupported ${parser} normalization_version`);
  assert(manifest.postprocess_version === spec.postprocessVersion, `Unsupported ${parser} postprocess_version`);
  validateTensorContract(parser, manifest);
}

const parseConfigLabels = (
  configText: string,
  requiredLabels: string[],
): {
  idToLabel: Record<number, string>;
  labelToId: Record<string, number>;
} => {
  const config = JSON.parse(configText) as {
    id2label?: Record<string, string>;
    label2id?: Record<string, number>;
  };

  assert(isObject(config.id2label), 'Artifact config is missing id2label');
  assert(isObject(config.label2id), 'Artifact config is missing label2id');

  const idToLabel = Object.fromEntries(
    Object.entries(config.id2label).map(([key, value]) => [Number(key), String(value)]),
  ) as Record<number, string>;
  const labelToId = Object.fromEntries(
    Object.entries(config.label2id).map(([key, value]) => [String(key), Number(value)]),
  ) as Record<string, number>;

  for (const label of requiredLabels) {
    assert(label in labelToId, `Artifact config is missing required label ${label}`);
  }

  return {
    idToLabel,
    labelToId,
  };
};

const readManifest = async (
  source: MlArtifactSource,
): Promise<{
  manifest: MlArtifactManifest;
  root: string;
}> => {
  if (source.kind === 'directory') {
    const manifestPath = path.join(source.path, 'manifest.json');
    const payload = await fs.readFile(manifestPath, 'utf8');
    return {
      manifest: parseManifest(payload),
      root: source.path,
    };
  }

  const manifestUrl = toManifestUrl(source);
  const payload = textDecoder.decode(await fetchBytes(manifestUrl));
  return {
    manifest: parseManifest(payload),
    root: manifestUrl,
  };
};

export function loadMlArtifact<TParser extends SupportedMlArtifactParser>(
  source: MlArtifactSource,
  expectedParser: TParser,
): Promise<LoadedMlArtifact<TParser>>;
export function loadMlArtifact(source: MlArtifactSource): Promise<LoadedMlArtifact>;
export async function loadMlArtifact(
  source: MlArtifactSource,
  expectedParser?: SupportedMlArtifactParser,
): Promise<LoadedMlArtifact> {
  const { manifest, root } = await readManifest(source);
  const parserCandidate: unknown = expectedParser ?? manifest.parser;
  assertSupportedArtifactParser(parserCandidate);
  const parser: SupportedMlArtifactParser = parserCandidate;
  validateManifestForParser(manifest, parser);

  const { artifact_id: _artifactId, ...artifactManifest } = manifest;
  const expectedArtifactId = computeArtifactId(artifactManifest);
  assert(expectedArtifactId === manifest.artifact_id, 'Artifact manifest artifact_id does not match bundle contents');

  const files = await loadArtifactFiles(source, root, manifest);
  const { idToLabel, labelToId } = parseConfigLabels(files.config.text || '', PARSER_RUNTIME_SPECS[parser].requiredLabels);
  const tokenizer = createBertWordPieceTokenizer(manifest.tokenizer, files);
  const labels = orderedLabels(idToLabel);
  const decoding = manifest.decoding ?? 'argmax';
  validateBioLabels(labels);
  const sequenceDecoder = files.sequence_decoder
    ? validateSequenceDecoderSidecar(JSON.parse(files.sequence_decoder.text || ''), labels)
    : undefined;

  return {
    parser,
    suite_version: manifest.suite_version,
    artifact_id: manifest.artifact_id,
    manifest: manifest as ArtifactManifestByParser[typeof parser],
    tokenizer,
    files,
    idToLabel,
    labelToId,
    labels,
    decoding,
    ...(sequenceDecoder ? { sequenceDecoder } : {}),
  };
}
