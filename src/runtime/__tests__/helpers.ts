/// <reference types="node" />

import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import {
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
  computeArtifactId,
} from '../index.ts';
import type {
  CalendarArtifactManifest,
  ReminderArtifactManifest,
  TaskArtifactManifest,
  TaskDecompositionArtifactManifest,
} from '../types.ts';

const DEFAULT_VOCAB = [
  '[PAD]',
  '[UNK]',
  '[CLS]',
  '[SEP]',
  '[MASK]',
  'lunch',
  'with',
  'sara',
  'every',
  'tuesday',
  'starting',
  'next',
  'month',
  'july',
  '1st',
  'design',
  'review',
  'tomorrow',
  'at',
  '3',
  'pm',
  'office',
  'unless',
  'it',
  "'",
  's',
  'a',
  'holiday',
  'then',
  'the',
  'following',
  'thursday',
];

export const CALENDAR_LABELS = [
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
  'B-description',
  'I-description',
];

const TASK_VOCAB = [
  '[PAD]',
  '[UNK]',
  '[CLS]',
  '[SEP]',
  '[MASK]',
  'finish',
  'report',
  'tomorrow',
  'at',
  '3',
  'pm',
  'with',
  'sara',
  'nina',
  'and',
  'omar',
  'project',
  'atlas',
  'high',
  'priority',
  '90',
  'minutes',
  'in',
  'progress',
  'tags',
  '#',
  'finance',
  'board',
  'every',
  'tuesday',
  'except',
  'the',
  'second',
  'of',
  'month',
  'starting',
  'today',
  'till',
  '24th',
  'august',
  'by',
  '10',
  'am',
];

const TASK_LABELS = [
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
];

const REMINDER_VOCAB = [
  '[PAD]',
  '[UNK]',
  '[CLS]',
  '[SEP]',
  '[MASK]',
  'water',
  'plants',
  'every',
  'other',
  'saturday',
  'except',
  'the',
  'week',
  'of',
  '21st',
  'and',
  'add',
  'soil',
  'call',
  'mom',
  'tomorrow',
  'at',
  '9',
  'am',
  'night',
  'attach',
  'mailers',
  'to',
  'phil',
  'wednesday',
  'for',
  '3',
  'months',
  'till',
  'august',
  'first',
  'holiday',
];

const REMINDER_LABELS = [
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
];

const TASK_DECOMPOSITION_VOCAB = [
  '[PAD]',
  '[UNK]',
  '[CLS]',
  '[SEP]',
  '[MASK]',
  'create',
  '2',
  'briefs',
  ',',
  'one',
  'for',
  'anthony',
  'and',
  'daliah',
  'by',
  '8',
  'pm',
  'next',
  'week',
  'plan',
  'launch',
  'docs',
  'tests',
  'changelog',
];

const TASK_DECOMPOSITION_LABELS = [
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
];

const sha256Hex = (input: string | Uint8Array): string =>
  createHash('sha256').update(typeof input === 'string' ? Buffer.from(input, 'utf8') : input).digest('hex');

const writeFile = async (dir: string, filename: string, content: string | Uint8Array): Promise<{
  path: string;
  sha256: string;
  size_bytes: number;
}> => {
  const absolutePath = path.join(dir, filename);
  const bytes = typeof content === 'string' ? Buffer.from(content, 'utf8') : Buffer.from(content);
  await fs.writeFile(absolutePath, bytes);
  return {
    path: filename,
    sha256: sha256Hex(bytes),
    size_bytes: bytes.byteLength,
  };
};

export const buildBioSequenceDecoderSidecar = (labels: string[]) => ({
  algorithm: 'BIO Viterbi',
  first_subword_piece_only: true,
  accumulator: 'float64',
  start_scores: labels.map((label) => (label.startsWith('I-') ? null : 0)),
  transition_scores_previous_by_next: labels.map((previous) =>
    labels.map((next) => (!next.startsWith('I-') || previous === next || previous === `B-${next.slice(2)}` ? 0 : null)),
  ),
  end_scores: labels.map(() => 0),
  null_score_means: 'negative infinity',
  tie_break: 'lowest predecessor ID, then lowest final ID',
  labels,
  runtime_max_tokens: 128,
});

export const createCalendarTestBundle = async (options?: {
  suiteVersion?: string;
  extraVocab?: string[];
  decoding?: 'argmax' | 'bio';
  sequenceDecoder?: unknown;
}): Promise<{
  dir: string;
  manifestPath: string;
  manifest: CalendarArtifactManifest;
  cleanup(): Promise<void>;
}> => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'kalandar-calendar-artifact-'));
  const vocab = [...DEFAULT_VOCAB, ...(options?.extraVocab || [])].filter(
    (token, index, source) => source.indexOf(token) === index,
  );
  const suiteVersion = options?.suiteVersion || 'suite-test-2026-04-18';

  const id2label = Object.fromEntries(CALENDAR_LABELS.map((label, index) => [String(index), label]));
  const label2id = Object.fromEntries(CALENDAR_LABELS.map((label, index) => [label, index]));

  const files = {
    model: await writeFile(dir, 'model.onnx', 'not-a-real-onnx-model'),
    config: await writeFile(
      dir,
      'config.json',
      `${JSON.stringify({ id2label, label2id, max_position_embeddings: 512 }, null, 2)}\n`,
    ),
    tokenizer_config: await writeFile(
      dir,
      'tokenizer_config.json',
      `${JSON.stringify(
        {
          tokenizer_class: 'BertTokenizer',
          do_lower_case: true,
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    special_tokens_map: await writeFile(
      dir,
      'special_tokens_map.json',
      `${JSON.stringify(
        {
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    vocab: await writeFile(dir, 'vocab.txt', `${vocab.join('\n')}\n`),
    tokenizer_json: await writeFile(
      dir,
      'tokenizer.json',
      `${JSON.stringify(
        {
          version: '1.0',
          model: { type: 'WordPiece' },
          normalizer: { type: 'BertNormalizer', lowercase: true, clean_text: true, handle_chinese_chars: true },
          pre_tokenizer: { type: 'BertPreTokenizer' },
          post_processor: { type: 'TemplateProcessing' },
        },
        null,
        2,
      )}\n`,
    ),
  };

  const manifestBase = {
    manifest_version: ARTIFACT_MANIFEST_VERSION,
    parser: 'calendar' as const,
    suite_version: suiteVersion,
    tensor_contract_version: CALENDAR_TENSOR_CONTRACT_VERSION,
    normalization_version: CALENDAR_NORMALIZATION_VERSION,
    postprocess_version: CALENDAR_POSTPROCESS_VERSION,
    tokenizer: {
      kind: 'bert_wordpiece' as const,
      do_lower_case: true,
      unk_token: '[UNK]',
      cls_token: '[CLS]',
      sep_token: '[SEP]',
      pad_token: '[PAD]',
      max_sequence_length: 512,
    },
    tensor_contract: {
      inputs: [
        { name: 'input_ids', dtype: 'int64' as const, rank: 2 },
        { name: 'attention_mask', dtype: 'int64' as const, rank: 2 },
      ],
      outputs: [{ name: 'logits', dtype: 'float32' as const, rank: 3 }],
      max_sequence_length: 512,
      padding: 'none' as const,
      truncation: 'tail' as const,
      empty_input_behavior: 'return_empty_result' as const,
      whitespace_input_behavior: 'return_empty_result' as const,
      oversized_input_behavior: 'truncate' as const,
    },
    ...(options?.decoding ? { decoding: options.decoding } : {}),
    files: {
      ...files,
      ...(options?.sequenceDecoder
        ? {
            sequence_decoder: await writeFile(dir, 'sequence_decoder.json', `${JSON.stringify(options.sequenceDecoder, null, 2)}\n`),
          }
        : {}),
    },
  };

  const manifest: CalendarArtifactManifest = {
    ...manifestBase,
    artifact_id: computeArtifactId(manifestBase),
  };

  const manifestPath = path.join(dir, 'manifest.json');
  await fs.writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

  return {
    dir,
    manifestPath,
    manifest,
    async cleanup() {
      await fs.rm(dir, { recursive: true, force: true });
    },
  };
};

export const createTaskTestBundle = async (options?: {
  suiteVersion?: string;
  extraVocab?: string[];
}): Promise<{
  dir: string;
  manifestPath: string;
  manifest: TaskArtifactManifest;
  cleanup(): Promise<void>;
}> => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'kalandar-task-artifact-'));
  const vocab = [...TASK_VOCAB, ...(options?.extraVocab || [])].filter((token, index, source) => source.indexOf(token) === index);
  const suiteVersion = options?.suiteVersion || 'suite-test-2026-04-19';

  const id2label = Object.fromEntries(TASK_LABELS.map((label, index) => [String(index), label]));
  const label2id = Object.fromEntries(TASK_LABELS.map((label, index) => [label, index]));

  const files = {
    model: await writeFile(dir, 'model.onnx', 'not-a-real-task-onnx-model'),
    config: await writeFile(
      dir,
      'config.json',
      `${JSON.stringify({ id2label, label2id, max_position_embeddings: 512 }, null, 2)}\n`,
    ),
    tokenizer_config: await writeFile(
      dir,
      'tokenizer_config.json',
      `${JSON.stringify(
        {
          tokenizer_class: 'BertTokenizer',
          do_lower_case: true,
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    special_tokens_map: await writeFile(
      dir,
      'special_tokens_map.json',
      `${JSON.stringify(
        {
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    vocab: await writeFile(dir, 'vocab.txt', `${vocab.join('\n')}\n`),
    tokenizer_json: await writeFile(
      dir,
      'tokenizer.json',
      `${JSON.stringify(
        {
          version: '1.0',
          model: { type: 'WordPiece' },
          normalizer: { type: 'BertNormalizer', lowercase: true, clean_text: true, handle_chinese_chars: true },
          pre_tokenizer: { type: 'BertPreTokenizer' },
          post_processor: { type: 'TemplateProcessing' },
        },
        null,
        2,
      )}\n`,
    ),
  };

  const manifestBase = {
    manifest_version: ARTIFACT_MANIFEST_VERSION,
    parser: 'task' as const,
    suite_version: suiteVersion,
    tensor_contract_version: TASK_TENSOR_CONTRACT_VERSION,
    normalization_version: TASK_NORMALIZATION_VERSION,
    postprocess_version: TASK_POSTPROCESS_VERSION,
    tokenizer: {
      kind: 'bert_wordpiece' as const,
      do_lower_case: true,
      unk_token: '[UNK]',
      cls_token: '[CLS]',
      sep_token: '[SEP]',
      pad_token: '[PAD]',
      max_sequence_length: 512,
    },
    tensor_contract: {
      inputs: [
        { name: 'input_ids', dtype: 'int64' as const, rank: 2 },
        { name: 'attention_mask', dtype: 'int64' as const, rank: 2 },
      ],
      outputs: [{ name: 'logits', dtype: 'float32' as const, rank: 3 }],
      max_sequence_length: 512,
      padding: 'none' as const,
      truncation: 'tail' as const,
      empty_input_behavior: 'return_empty_result' as const,
      whitespace_input_behavior: 'return_empty_result' as const,
      oversized_input_behavior: 'truncate' as const,
    },
    files,
  };

  const manifest: TaskArtifactManifest = {
    ...manifestBase,
    artifact_id: computeArtifactId(manifestBase),
  };

  const manifestPath = path.join(dir, 'manifest.json');
  await fs.writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

  return {
    dir,
    manifestPath,
    manifest,
    async cleanup() {
      await fs.rm(dir, { recursive: true, force: true });
    },
  };
};

export const createReminderTestBundle = async (options?: {
  suiteVersion?: string;
  extraVocab?: string[];
}): Promise<{
  dir: string;
  manifestPath: string;
  manifest: ReminderArtifactManifest;
  cleanup(): Promise<void>;
}> => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'kalandar-reminder-artifact-'));
  const vocab = [...REMINDER_VOCAB, ...(options?.extraVocab || [])].filter(
    (token, index, source) => source.indexOf(token) === index,
  );
  const suiteVersion = options?.suiteVersion || 'suite-test-2026-04-19';

  const id2label = Object.fromEntries(REMINDER_LABELS.map((label, index) => [String(index), label]));
  const label2id = Object.fromEntries(REMINDER_LABELS.map((label, index) => [label, index]));

  const files = {
    model: await writeFile(dir, 'model.onnx', 'not-a-real-reminder-onnx-model'),
    config: await writeFile(
      dir,
      'config.json',
      `${JSON.stringify({ id2label, label2id, max_position_embeddings: 512 }, null, 2)}\n`,
    ),
    tokenizer_config: await writeFile(
      dir,
      'tokenizer_config.json',
      `${JSON.stringify(
        {
          tokenizer_class: 'BertTokenizer',
          do_lower_case: true,
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    special_tokens_map: await writeFile(
      dir,
      'special_tokens_map.json',
      `${JSON.stringify(
        {
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    vocab: await writeFile(dir, 'vocab.txt', `${vocab.join('\n')}\n`),
    tokenizer_json: await writeFile(
      dir,
      'tokenizer.json',
      `${JSON.stringify(
        {
          version: '1.0',
          model: { type: 'WordPiece' },
          normalizer: { type: 'BertNormalizer', lowercase: true, clean_text: true, handle_chinese_chars: true },
          pre_tokenizer: { type: 'BertPreTokenizer' },
          post_processor: { type: 'TemplateProcessing' },
        },
        null,
        2,
      )}\n`,
    ),
  };

  const manifestBase = {
    manifest_version: ARTIFACT_MANIFEST_VERSION,
    parser: 'reminder' as const,
    suite_version: suiteVersion,
    tensor_contract_version: REMINDER_TENSOR_CONTRACT_VERSION,
    normalization_version: REMINDER_NORMALIZATION_VERSION,
    postprocess_version: REMINDER_POSTPROCESS_VERSION,
    tokenizer: {
      kind: 'bert_wordpiece' as const,
      do_lower_case: true,
      unk_token: '[UNK]',
      cls_token: '[CLS]',
      sep_token: '[SEP]',
      pad_token: '[PAD]',
      max_sequence_length: 512,
    },
    tensor_contract: {
      inputs: [
        { name: 'input_ids', dtype: 'int64' as const, rank: 2 },
        { name: 'attention_mask', dtype: 'int64' as const, rank: 2 },
      ],
      outputs: [{ name: 'logits', dtype: 'float32' as const, rank: 3 }],
      max_sequence_length: 512,
      padding: 'none' as const,
      truncation: 'tail' as const,
      empty_input_behavior: 'return_empty_result' as const,
      whitespace_input_behavior: 'return_empty_result' as const,
      oversized_input_behavior: 'truncate' as const,
    },
    files,
  };

  const manifest: ReminderArtifactManifest = {
    ...manifestBase,
    artifact_id: computeArtifactId(manifestBase),
  };

  const manifestPath = path.join(dir, 'manifest.json');
  await fs.writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

  return {
    dir,
    manifestPath,
    manifest,
    async cleanup() {
      await fs.rm(dir, { recursive: true, force: true });
    },
  };
};

export const createTaskDecompositionTestBundle = async (options?: {
  suiteVersion?: string;
  extraVocab?: string[];
}): Promise<{
  dir: string;
  manifestPath: string;
  manifest: TaskDecompositionArtifactManifest;
  cleanup(): Promise<void>;
}> => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'kalandar-task-decomposition-artifact-'));
  const vocab = [...TASK_DECOMPOSITION_VOCAB, ...(options?.extraVocab || [])].filter(
    (token, index, source) => source.indexOf(token) === index,
  );
  const suiteVersion = options?.suiteVersion || 'suite-test-2026-04-24';

  const id2label = Object.fromEntries(TASK_DECOMPOSITION_LABELS.map((label, index) => [String(index), label]));
  const label2id = Object.fromEntries(TASK_DECOMPOSITION_LABELS.map((label, index) => [label, index]));

  const files = {
    model: await writeFile(dir, 'model.onnx', 'not-a-real-task-decomposition-onnx-model'),
    config: await writeFile(
      dir,
      'config.json',
      `${JSON.stringify({ id2label, label2id, max_position_embeddings: 512 }, null, 2)}\n`,
    ),
    tokenizer_config: await writeFile(
      dir,
      'tokenizer_config.json',
      `${JSON.stringify(
        {
          tokenizer_class: 'BertTokenizer',
          do_lower_case: true,
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    special_tokens_map: await writeFile(
      dir,
      'special_tokens_map.json',
      `${JSON.stringify(
        {
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
        },
        null,
        2,
      )}\n`,
    ),
    vocab: await writeFile(dir, 'vocab.txt', `${vocab.join('\n')}\n`),
    tokenizer_json: await writeFile(
      dir,
      'tokenizer.json',
      `${JSON.stringify(
        {
          version: '1.0',
          model: { type: 'WordPiece' },
          normalizer: { type: 'BertNormalizer', lowercase: true, clean_text: true, handle_chinese_chars: true },
          pre_tokenizer: { type: 'BertPreTokenizer' },
          post_processor: { type: 'TemplateProcessing' },
        },
        null,
        2,
      )}\n`,
    ),
  };

  const manifestBase = {
    manifest_version: ARTIFACT_MANIFEST_VERSION,
    parser: 'task_decomposition' as const,
    suite_version: suiteVersion,
    tensor_contract_version: TASK_DECOMPOSITION_TENSOR_CONTRACT_VERSION,
    normalization_version: TASK_DECOMPOSITION_NORMALIZATION_VERSION,
    postprocess_version: TASK_DECOMPOSITION_POSTPROCESS_VERSION,
    tokenizer: {
      kind: 'bert_wordpiece' as const,
      do_lower_case: true,
      unk_token: '[UNK]',
      cls_token: '[CLS]',
      sep_token: '[SEP]',
      pad_token: '[PAD]',
      max_sequence_length: 512,
    },
    tensor_contract: {
      inputs: [
        { name: 'input_ids', dtype: 'int64' as const, rank: 2 },
        { name: 'attention_mask', dtype: 'int64' as const, rank: 2 },
      ],
      outputs: [{ name: 'logits', dtype: 'float32' as const, rank: 3 }],
      max_sequence_length: 512,
      padding: 'none' as const,
      truncation: 'tail' as const,
      empty_input_behavior: 'return_empty_result' as const,
      whitespace_input_behavior: 'return_empty_result' as const,
      oversized_input_behavior: 'truncate' as const,
    },
    files,
  };

  const manifest: TaskDecompositionArtifactManifest = {
    ...manifestBase,
    artifact_id: computeArtifactId(manifestBase),
  };

  const manifestPath = path.join(dir, 'manifest.json');
  await fs.writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

  return {
    dir,
    manifestPath,
    manifest,
    async cleanup() {
      await fs.rm(dir, { recursive: true, force: true });
    },
  };
};
