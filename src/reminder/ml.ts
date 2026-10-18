import { captureSchedule } from '../dates/index.ts';
import type { ScheduleSpan } from '../dates/index.ts';
import { finalizeReminderParseContext } from './finalize.ts';
import { titlePass } from './passes/titlePass.ts';
import { applyReminderSchedule } from './schedule.ts';
import type { ReminderFields, ReminderParseContext, ReminderParseOptions, ReminderParseResult } from './types.ts';
import { createReminderParseContext, extractTitle } from './utils.ts';
import { decodeFirstPieceSpans } from '../runtime/sequence-decoder.ts';
import type { InferenceAdapter, LoadedMlArtifact, MlArtifactSource, RuntimeTensor, SequenceDecoding } from '../runtime/types.ts';

const EXCEPTION_FRAGMENT_LABELS = new Set(['recurrence_exception', 'trigger_time']);
const EXCEPTION_START_REGEX = /^(?:except|unless|but skip|but not|skip\b)/i;
const EXCEPTION_CONTINUATION_REGEX =
  /^(?:the\s+week of the\b|week of the\b|the\s+first\b|first\b|second\b|third\b|last\b|holiday\b|holidays\b)/i;
const EXCEPTION_PREFIX_REGEX = /(?:except(?:\s+for)?|unless|but\s+skip|but\s+not|skip)\s*$/i;
const LEADING_TRAILING_TRIM_REGEX = /^[\s,;:.]+|[\s,;:.]+$/g;
const SCHEDULE_LABELS = new Set([
  'trigger_time',
  'remind_at',
  'lead_time',
  'recurrence',
  'recurrence_start',
  'recurrence_end',
  'recurrence_exception',
  'recurrence_window',
]);

type ConfidenceField = keyof ReminderParseResult['meta']['confidence'];
type SpanField = keyof ReminderParseResult['meta']['spans'];

export interface ReminderMlEntity {
  label: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
}

export type ReminderParserConfig =
  | { mode?: 'lightweight' }
  | {
      mode: 'ml';
      artifact: MlArtifactSource | LoadedMlArtifact<'reminder'>;
      inference: InferenceAdapter;
    };

export interface ReminderParserInfo {
  parser: 'reminder';
  suite_version?: string;
  artifact_id?: string;
}

export interface ReminderParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: ReminderParserInfo;
  parse(input: string, opts?: ReminderParseOptions): Promise<ReminderParseResult>;
  dispose(): Promise<void>;
}

const cloneValue = <T>(value: T): T =>
  typeof globalThis.structuredClone === 'function'
    ? globalThis.structuredClone(value)
    : (JSON.parse(JSON.stringify(value)) as T);

const buildTensor = (shape: number[], dtype: RuntimeTensor['dtype'], data: number[]): RuntimeTensor => ({
  shape,
  dtype,
  data,
});

export const decodeReminderMlEntities = (
  text: string,
  offsets: Array<[number, number]>,
  logits: RuntimeTensor,
  labels: string[],
  decoding: SequenceDecoding,
): ReminderMlEntity[] => {
  const [batch, sequenceLength, labelCount] = logits.shape;
  if (batch !== 1) {
    throw new Error(`Reminder logits tensor must have batch size 1, received ${batch}`);
  }
  if (sequenceLength !== offsets.length) {
    throw new Error(`Reminder logits sequence length mismatch: expected ${offsets.length}, received ${sequenceLength}`);
  }
  if (labelCount !== labels.length) {
    throw new Error(`Reminder logits label dimension mismatch: expected ${labels.length}, received ${labelCount}`);
  }

  return decodeFirstPieceSpans({ text, offsets, logits: logits.data, labels, decoding }).map(({ label, text: spanText, start, end, confidence }) => ({
    label,
    text: spanText,
    start,
    end,
    confidence,
  }));
};

const trimReminderEntity = (entity: ReminderMlEntity): ReminderMlEntity | null => {
  const leadingMatch = entity.text.match(/^[\s,;:.]*/);
  const trailingMatch = entity.text.match(/[\s,;:.]*$/);
  const leading = leadingMatch?.[0].length || 0;
  const trailing = trailingMatch?.[0].length || 0;
  const trimmed = entity.text.replace(LEADING_TRAILING_TRIM_REGEX, '');
  if (!trimmed) {
    return null;
  }

  const start = entity.start + leading;
  const end = entity.end - trailing;
  return {
    ...entity,
    text: trimmed,
    start,
    end,
  };
};

const sortEntities = (entities: ReminderMlEntity[]): ReminderMlEntity[] =>
  [...entities].sort((left, right) => left.start - right.start || left.end - right.end || left.label.localeCompare(right.label));

const shouldExpandExceptionPrefix = (text: string, entities: ReminderMlEntity[], index: number): boolean => {
  const entity = entities[index];
  if (entity.label !== 'recurrence_exception') {
    return false;
  }
  if (EXCEPTION_START_REGEX.test(entity.text)) {
    return false;
  }

  const prefixWindowStart = Math.max(0, entity.start - 16);
  const prefix = text.slice(prefixWindowStart, entity.start);
  if (!EXCEPTION_PREFIX_REGEX.test(prefix)) {
    return false;
  }

  if (!['the', 'a', 'an'].includes(entity.text.trim().toLowerCase())) {
    return true;
  }
  if (index + 1 >= entities.length) {
    return false;
  }

  const nextEntity = entities[index + 1];
  return nextEntity.label === 'recurrence' && EXCEPTION_CONTINUATION_REGEX.test(nextEntity.text);
};

const expandExceptionPrefix = (
  text: string,
  entities: ReminderMlEntity[],
  index: number,
): { entity: ReminderMlEntity; nextIndex: number } => {
  const entity = entities[index];
  const prefixWindowStart = Math.max(0, entity.start - 16);
  const prefix = text.slice(prefixWindowStart, entity.start);
  const match = prefix.match(EXCEPTION_PREFIX_REGEX);
  const start = match ? prefixWindowStart + prefix.lastIndexOf(match[0]) : entity.start;
  let end = entity.end;
  let confidenceTotal = entity.confidence;
  let confidenceCount = 1;
  let cursor = index + 1;

  while (cursor < entities.length) {
    const candidate = entities[cursor];
    if (candidate.start - end > 3) {
      break;
    }
    if (candidate.label === 'recurrence' && EXCEPTION_CONTINUATION_REGEX.test(candidate.text)) {
      end = candidate.end;
      confidenceTotal += candidate.confidence;
      confidenceCount += 1;
      cursor += 1;
      continue;
    }
    if (EXCEPTION_FRAGMENT_LABELS.has(candidate.label)) {
      end = candidate.end;
      confidenceTotal += candidate.confidence;
      confidenceCount += 1;
      cursor += 1;
      continue;
    }
    break;
  }

  return {
    entity: {
      label: 'recurrence_exception',
      start,
      end,
      text: text.slice(start, end),
      confidence: Number((confidenceTotal / Math.max(confidenceCount, 1)).toFixed(4)),
    },
    nextIndex: cursor,
  };
};

const mergeReminderExceptionFragments = (text: string, entities: ReminderMlEntity[]): ReminderMlEntity[] => {
  const result: ReminderMlEntity[] = [];
  let index = 0;

  while (index < entities.length) {
    const entity = entities[index];

    if (shouldExpandExceptionPrefix(text, entities, index)) {
      const expanded = expandExceptionPrefix(text, entities, index);
      const trimmed = trimReminderEntity(expanded.entity);
      if (trimmed) {
        result.push(trimmed);
      }
      index = expanded.nextIndex;
      continue;
    }

    if (entity.label !== 'recurrence_exception' || !EXCEPTION_START_REGEX.test(entity.text)) {
      result.push(entity);
      index += 1;
      continue;
    }

    let end = entity.end;
    let confidenceTotal = entity.confidence;
    let confidenceCount = 1;
    let cursor = index + 1;

    while (cursor < entities.length) {
      const candidate = entities[cursor];
      if (!EXCEPTION_FRAGMENT_LABELS.has(candidate.label)) {
        break;
      }
      if (candidate.start - end > 3) {
        break;
      }
      end = candidate.end;
      confidenceTotal += candidate.confidence;
      confidenceCount += 1;
      cursor += 1;
    }

    const trimmed = trimReminderEntity({
      label: 'recurrence_exception',
      start: entity.start,
      end,
      text: text.slice(entity.start, end),
      confidence: Number((confidenceTotal / Math.max(confidenceCount, 1)).toFixed(4)),
    });
    if (trimmed) {
      result.push(trimmed);
    }
    index = cursor;
  }

  return result;
};

export const mergeReminderMlTitleValue = (text: string, entities: ReminderMlEntity[]): string | null => {
  const titles = sortEntities(entities.filter((entity) => entity.label === 'title'));
  if (titles.length === 0) {
    return null;
  }

  const parts = [titles[0].text];
  for (let index = 1; index < titles.length; index += 1) {
    const previous = titles[index - 1];
    const current = titles[index];
    const gap = text.slice(previous.end, current.start);
    let joiner = ' ';
    if (/\band\s+$/i.test(gap)) {
      joiner = ' and ';
    } else if (/\bor\s+$/i.test(gap)) {
      joiner = ' or ';
    }
    parts.push(`${joiner}${current.text}`);
  }

  return parts.join('').trim();
};

export const postprocessReminderMlEntities = (text: string, entities: ReminderMlEntity[]): ReminderMlEntity[] => {
  const trimmed = sortEntities(
    entities
      .map((entity) => trimReminderEntity(entity))
      .filter((entity): entity is ReminderMlEntity => Boolean(entity)),
  );
  return mergeReminderExceptionFragments(text, trimmed);
};

const appendDebugSteps = (ctx: ReminderParseContext, steps: ReminderParseResult['meta']['debugSteps']): void => {
  const seen = new Set(ctx.result.meta.debugSteps.map((step) => `${step.pass}|${step.ruleId}|${step.start}|${step.end}|${step.text}`));
  for (const step of steps) {
    const key = `${step.pass}|${step.ruleId}|${step.start}|${step.end}|${step.text}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    ctx.result.meta.debugSteps.push(cloneValue(step));
  }
};

const hasFieldValue = (value: unknown): boolean => {
  if (Array.isArray(value)) {
    return value.length > 0;
  }
  if (typeof value === 'string') {
    return value.trim().length > 0;
  }
  return value !== null && value !== undefined;
};

const applyFieldValue = <TField extends keyof ReminderFields>(
  ctx: ReminderParseContext,
  field: TField,
  value: ReminderFields[TField],
  confidence: number,
  spans: ReminderParseResult['meta']['spans'][TField],
): boolean => {
  if (!hasFieldValue(value)) {
    return false;
  }

  const currentValue = ctx.result.fields[field];
  const currentConfidence = ctx.result.meta.confidence[field as ConfidenceField];
  const shouldReplace = !hasFieldValue(currentValue) || confidence >= currentConfidence;
  if (!shouldReplace) {
    return false;
  }

  ctx.result.fields[field] = cloneValue(value);
  ctx.result.meta.confidence[field as ConfidenceField] = confidence;
  ctx.result.meta.spans[field as SpanField] = cloneValue(spans);
  return true;
};

const maskReminderSpan = (ctx: ReminderParseContext, start: number, end: number): void => {
  if (end <= start || start < 0 || end > ctx.maskedInput.length) {
    return;
  }

  const before = ctx.maskedInput.slice(0, start);
  const middle = ' '.repeat(end - start);
  const after = ctx.maskedInput.slice(end);
  ctx.maskedInput = `${before}${middle}${after}`;
  ctx.result.meta.maskedInput = ctx.maskedInput;
};

const entitySpan = (entity: ReminderMlEntity) => ({
  start: entity.start,
  end: entity.end,
  text: entity.text,
});

const normalizeLocationText = (text: string): string =>
  text
    .trim()
    .replace(/^(?:at|in|on|@)\s+/i, '')
    .replace(/^["']+|["']+$/g, '')
    .replace(/[ ,;:.]+$/g, '')
    .replace(/\s+/g, ' ')
    .trim();

const applyScheduleEntities = (ctx: ReminderParseContext, entities: ReminderMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const found = captureSchedule(ctx.rawInput, 'reminder', {
    now: ctx.now,
    timezone: ctx.options.timezone,
    spans: entities.map((entity) => ({ label: entity.label, start: entity.start, end: entity.end })),
  });
  const spanConfidence = (span: ScheduleSpan): number | null => {
    const overlapping = entities.filter((entity) => entity.start < span.end && span.start < entity.end);
    return overlapping.length > 0 ? Math.max(...overlapping.map((entity) => entity.confidence)) : null;
  };
  applyReminderSchedule(ctx, found, 'ml', spanConfidence);
};

const applyLocationEntities = (ctx: ReminderParseContext, entities: ReminderMlEntity[]): void => {
  const sorted = sortEntities(entities).sort((left, right) => right.confidence - left.confidence);
  for (const entity of sorted) {
    const location = normalizeLocationText(entity.text);
    if (!location) {
      continue;
    }

    const applied = applyFieldValue(ctx, 'location', location, entity.confidence, [entitySpan(entity)]);
    if (!applied) {
      continue;
    }

    appendDebugSteps(ctx, [
      {
        pass: 'ml',
        ruleId: 'ml.location_entity',
        start: entity.start,
        end: entity.end,
        text: entity.text,
        confidence: entity.confidence,
        note: 'location assembled from decoded ML entity span',
      },
    ]);
    return;
  }
};

const applyTitleFromEntities = (ctx: ReminderParseContext, entities: ReminderMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const mergedTitle = mergeReminderMlTitleValue(ctx.rawInput, entities);
  if (!mergedTitle) {
    return;
  }

  const title = extractTitle(mergedTitle);
  const confidence = Math.max(...entities.map((entity) => entity.confidence), 0);
  const applied = applyFieldValue(
    ctx,
    'title',
    title,
    Math.max(confidence, ctx.result.meta.confidence.title),
    entities.map((entity) => entitySpan(entity)),
  );
  if (!applied) {
    return;
  }

  appendDebugSteps(ctx, [
    {
      pass: 'ml',
      ruleId: 'ml.title_entities',
      start: entities[0].start,
      end: entities[entities.length - 1].end,
      text: mergedTitle,
      confidence,
      note: 'title merged from decoded ML title spans',
    },
  ]);
};

const validateLogitsTensor = (artifact: LoadedMlArtifact<'reminder'>, encodingLength: number, tensor: RuntimeTensor): void => {
  if (tensor.dtype !== 'float32') {
    throw new Error(`Reminder logits tensor must be float32, received ${tensor.dtype}`);
  }
  if (tensor.shape.length !== 3) {
    throw new Error(`Reminder logits tensor must be rank 3, received rank ${tensor.shape.length}`);
  }
  if (tensor.shape[0] !== 1) {
    throw new Error(`Reminder logits tensor must have batch size 1, received ${tensor.shape[0]}`);
  }
  if (tensor.shape[1] !== encodingLength) {
    throw new Error(`Reminder logits tensor sequence length mismatch: expected ${encodingLength}, received ${tensor.shape[1]}`);
  }
  if (tensor.shape[2] !== Object.keys(artifact.idToLabel).length) {
    throw new Error(
      `Reminder logits tensor label dimension mismatch: expected ${Object.keys(artifact.idToLabel).length}, received ${tensor.shape[2]}`,
    );
  }
};

export const parseReminderInputWithMl = async (
  input: string,
  opts: ReminderParseOptions | undefined,
  artifact: LoadedMlArtifact<'reminder'>,
  inference: InferenceAdapter,
): Promise<ReminderParseResult> => {
  const ctx = createReminderParseContext(input, opts);
  ctx.result.meta.runtime = {
    mode: 'ml',
    suite_version: artifact.suite_version,
    artifact_id: artifact.artifact_id,
  };

  if (!ctx.rawInput) {
    titlePass(ctx);
    return finalizeReminderParseContext(ctx);
  }

  const encoding = artifact.tokenizer.encode(ctx.rawInput);
  const request = {
    parser: 'reminder' as const,
    artifact_id: artifact.artifact_id,
    suite_version: artifact.suite_version,
    tensor_contract_version: artifact.manifest.tensor_contract_version,
    inputs: {
      input_ids: buildTensor([1, encoding.inputIds.length], 'int64', encoding.inputIds),
      attention_mask: buildTensor([1, encoding.attentionMask.length], 'int64', encoding.attentionMask),
    },
    outputs: ['logits'],
  };

  const response = await inference.run(request);
  const logits = response.outputs.logits;
  if (!logits) {
    throw new Error('Reminder inference adapter did not return logits');
  }
  validateLogitsTensor(artifact, encoding.inputIds.length, logits);

  const decoded = decodeReminderMlEntities(ctx.rawInput, encoding.offsets, logits, artifact.labels, artifact.decoding);
  const entities = postprocessReminderMlEntities(ctx.rawInput, decoded);

  applyScheduleEntities(ctx, entities.filter((entity) => SCHEDULE_LABELS.has(entity.label)));
  applyLocationEntities(ctx, entities.filter((entity) => entity.label === 'location'));

  for (const entity of entities) {
    if (entity.label === 'title') {
      continue;
    }
    maskReminderSpan(ctx, entity.start, entity.end);
  }

  ctx.working = ctx.maskedInput;
  titlePass(ctx);
  if (entities.some((entity) => entity.label === 'title')) {
    applyTitleFromEntities(ctx, entities.filter((entity) => entity.label === 'title'));
  }

  if (encoding.truncated) {
    ctx.result.warnings = [
      ...(ctx.result.warnings || []),
      {
        code: 'ml_input_truncated',
        severity: 'warning',
        message: 'Input exceeded the reminder ML maximum sequence length and was truncated.',
        fieldHints: ['title'],
        spans: [],
        details: {
          maxSequenceLength: artifact.manifest.tensor_contract.max_sequence_length,
        },
      },
    ];
  }

  return finalizeReminderParseContext(ctx);
};
