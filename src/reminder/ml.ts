import { absoluteTimePass } from './passes/absoluteTimePass.ts';
import { chronoFallbackPass } from './passes/chronoFallbackPass.ts';
import { finalizeReminderParseContext } from './finalize.ts';
import { namedDatePass } from './passes/namedDatePass.ts';
import { recurrencePass } from './passes/recurrencePass.ts';
import { relativeTimePass } from './passes/relativeTimePass.ts';
import { scheduleDetailsPass } from './passes/scheduleDetailsPass.ts';
import { titlePass } from './passes/titlePass.ts';
import type { ReminderFields, ReminderParseContext, ReminderParseOptions, ReminderParseResult } from './types.ts';
import { createReminderParseContext, extractTitle } from './utils.ts';
import type { InferenceAdapter, LoadedMlArtifact, MlArtifactSource, RuntimeTensor } from '../runtime/types.ts';

const EXCEPTION_FRAGMENT_LABELS = new Set(['recurrence_exception', 'trigger_time']);
const EXCEPTION_START_REGEX = /^(?:except|unless|but skip|but not|skip\b)/i;
const EXCEPTION_CONTINUATION_REGEX =
  /^(?:the\s+week of the\b|week of the\b|the\s+first\b|first\b|second\b|third\b|last\b|holiday\b|holidays\b)/i;
const EXCEPTION_PREFIX_REGEX = /(?:except(?:\s+for)?|unless|but\s+skip|but\s+not|skip)\s*$/i;
const LEADING_TRAILING_TRIM_REGEX = /^[\s,;:.]+|[\s,;:.]+$/g;

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

const softmaxConfidence = (row: number[], selectedIndex: number): number => {
  const max = Math.max(...row);
  const exps = row.map((value) => Math.exp(value - max));
  const sum = exps.reduce((total, value) => total + value, 0);
  if (!Number.isFinite(sum) || sum <= 0) {
    return 0;
  }
  return exps[selectedIndex] / sum;
};

const finalizeEntity = (
  text: string,
  current: {
    label: string;
    start: number;
    end: number;
    confidenceSum: number;
    confidenceCount: number;
  },
): ReminderMlEntity => ({
  label: current.label,
  text: text.slice(current.start, current.end),
  start: current.start,
  end: current.end,
  confidence: Number((current.confidenceSum / Math.max(current.confidenceCount, 1)).toFixed(4)),
});

export const decodeReminderMlEntities = (
  text: string,
  inputIds: number[],
  offsets: Array<[number, number]>,
  logits: RuntimeTensor,
  idToLabel: Record<number, string>,
  specialTokenIds: Set<number>,
): ReminderMlEntity[] => {
  const [batch, sequenceLength, labelCount] = logits.shape;
  if (batch !== 1) {
    throw new Error(`Reminder logits tensor must have batch size 1, received ${batch}`);
  }
  if (sequenceLength !== inputIds.length) {
    throw new Error(`Reminder logits sequence length mismatch: expected ${inputIds.length}, received ${sequenceLength}`);
  }
  if (!Number.isInteger(labelCount) || labelCount <= 0) {
    throw new Error('Reminder logits tensor is missing a valid label dimension');
  }

  const entities: ReminderMlEntity[] = [];
  let current:
    | {
        label: string;
        start: number;
        end: number;
        confidenceSum: number;
        confidenceCount: number;
      }
    | null = null;

  for (let index = 0; index < sequenceLength; index += 1) {
    if (specialTokenIds.has(inputIds[index])) {
      continue;
    }

    const [start, end] = offsets[index] || [0, 0];
    if (end <= start) {
      continue;
    }

    const rowOffset = index * labelCount;
    const row = logits.data.slice(rowOffset, rowOffset + labelCount);
    let bestIndex = 0;
    for (let labelIndex = 1; labelIndex < row.length; labelIndex += 1) {
      if (row[labelIndex] > row[bestIndex]) {
        bestIndex = labelIndex;
      }
    }

    const rawLabel = idToLabel[bestIndex];
    if (!rawLabel || rawLabel === 'O') {
      if (current) {
        entities.push(finalizeEntity(text, current));
        current = null;
      }
      continue;
    }

    const [prefix, label] = rawLabel.split('-', 2);
    if (!prefix || !label) {
      if (current) {
        entities.push(finalizeEntity(text, current));
        current = null;
      }
      continue;
    }

    const confidence = softmaxConfidence(row, bestIndex);
    if (prefix === 'B' || !current || current.label !== label) {
      if (current) {
        entities.push(finalizeEntity(text, current));
      }
      current = {
        label,
        start,
        end,
        confidenceSum: confidence,
        confidenceCount: 1,
      };
      continue;
    }

    current.end = end;
    current.confidenceSum += confidence;
    current.confidenceCount += 1;
  }

  if (current) {
    entities.push(finalizeEntity(text, current));
  }

  return entities;
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

const createMaskedEntityContext = (
  input: string,
  opts: ReminderParseOptions | undefined,
  entities: ReminderMlEntity[],
): ReminderParseContext => {
  const ctx = createReminderParseContext(input, opts);
  const maskedChars = ctx.rawInput.split('').map(() => ' ');

  for (const entity of entities) {
    for (let index = entity.start; index < entity.end && index < maskedChars.length; index += 1) {
      maskedChars[index] = ctx.rawInput[index];
    }
  }

  const maskedInput = maskedChars.join('');
  ctx.working = maskedInput;
  ctx.maskedInput = maskedInput;
  ctx.result.meta.maskedInput = maskedInput;
  return ctx;
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

const mergeFieldFromContext = <TField extends keyof ReminderFields>(
  ctx: ReminderParseContext,
  sourceCtx: ReminderParseContext,
  field: TField,
): boolean => {
  const updated = applyFieldValue(
    ctx,
    field,
    sourceCtx.result.fields[field],
    sourceCtx.result.meta.confidence[field as ConfidenceField],
    sourceCtx.result.meta.spans[field as SpanField] as ReminderParseResult['meta']['spans'][TField],
  );
  if (updated) {
    appendDebugSteps(ctx, sourceCtx.result.meta.debugSteps);
  }
  return updated;
};

const mergeReminderState = (ctx: ReminderParseContext, sourceCtx: ReminderParseContext): void => {
  if (sourceCtx.state.anchorDayOfMonth !== null) {
    ctx.state.anchorDayOfMonth = sourceCtx.state.anchorDayOfMonth;
  }
  if (sourceCtx.state.anchorMonthDay) {
    ctx.state.anchorMonthDay = cloneValue(sourceCtx.state.anchorMonthDay);
  }
  if (sourceCtx.state.recurrenceStartDate) {
    ctx.state.recurrenceStartDate = sourceCtx.state.recurrenceStartDate;
  }
  if (sourceCtx.state.ambiguousTime) {
    ctx.state.ambiguousTime = true;
  }
  if (sourceCtx.state.explicitHour !== null) {
    ctx.state.explicitHour = sourceCtx.state.explicitHour;
  }
  if (sourceCtx.state.explicitMinute !== null) {
    ctx.state.explicitMinute = sourceCtx.state.explicitMinute;
  }
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

const normalizeContextHint = (
  text: string,
): { value: string; hour: number | null; minute: number | null } => {
  const cleaned = text.trim().replace(/[ ,;:.]+$/g, '');
  if (/^(?:at\s+)?morning$/i.test(cleaned)) {
    return { value: 'morning', hour: 9, minute: 0 };
  }
  if (/^(?:at\s+)?noon$/i.test(cleaned)) {
    return { value: 'noon', hour: 12, minute: 0 };
  }
  if (/^(?:at\s+)?afternoon$/i.test(cleaned)) {
    return { value: 'afternoon', hour: 15, minute: 0 };
  }
  if (/^(?:at\s+)?evening$/i.test(cleaned)) {
    return { value: 'evening', hour: 18, minute: 0 };
  }
  if (/^(?:at\s+)?night$/i.test(cleaned)) {
    return { value: 'night', hour: 20, minute: 0 };
  }
  if (/^first thing(?: in the morning)?$/i.test(cleaned)) {
    return { value: 'first thing in the morning', hour: 9, minute: 0 };
  }
  if (/^after lunch$/i.test(cleaned)) {
    return { value: 'after lunch', hour: 13, minute: 0 };
  }
  if (/^(?:end of day|eod|before i leave today)$/i.test(cleaned)) {
    return { value: 'end of day', hour: 17, minute: 0 };
  }
  return { value: cleaned.replace(/^at\s+/i, '').trim(), hour: null, minute: null };
};

const applyScheduleEntities = (
  ctx: ReminderParseContext,
  input: string,
  opts: ReminderParseOptions | undefined,
  entities: ReminderMlEntity[],
): void => {
  if (entities.length === 0) {
    return;
  }

  const subctx = createMaskedEntityContext(input, opts, entities);
  recurrencePass(subctx);
  scheduleDetailsPass(subctx);

  mergeFieldFromContext(ctx, subctx, 'recurrence');
  mergeFieldFromContext(ctx, subctx, 'recurrence_start');
  mergeFieldFromContext(ctx, subctx, 'recurrence_end');
  mergeFieldFromContext(ctx, subctx, 'recurrence_exception');
  mergeFieldFromContext(ctx, subctx, 'recurrence_window');
  mergeFieldFromContext(ctx, subctx, 'context_hint');
  mergeReminderState(ctx, subctx);
};

const applyTriggerTimeEntities = (
  ctx: ReminderParseContext,
  input: string,
  opts: ReminderParseOptions | undefined,
  entities: ReminderMlEntity[],
): void => {
  if (entities.length === 0) {
    return;
  }

  const subctx = createMaskedEntityContext(input, opts, entities);
  relativeTimePass(subctx);
  absoluteTimePass(subctx);
  namedDatePass(subctx);
  chronoFallbackPass(subctx);

  mergeFieldFromContext(ctx, subctx, 'remind_at');
  mergeFieldFromContext(ctx, subctx, 'context_hint');
  mergeReminderState(ctx, subctx);
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

const applyLeadTimeEntities = (ctx: ReminderParseContext, entities: ReminderMlEntity[]): void => {
  const sorted = sortEntities(entities).sort((left, right) => right.confidence - left.confidence);
  for (const entity of sorted) {
    const normalized = normalizeContextHint(entity.text);
    if (!normalized.value) {
      continue;
    }

    const applied = applyFieldValue(ctx, 'context_hint', normalized.value, entity.confidence, [entitySpan(entity)]);
    if (!applied) {
      continue;
    }

    if (normalized.hour !== null) {
      ctx.state.explicitHour = normalized.hour;
    }
    if (normalized.minute !== null) {
      ctx.state.explicitMinute = normalized.minute;
    }

    appendDebugSteps(ctx, [
      {
        pass: 'ml',
        ruleId: 'ml.lead_time_entity',
        start: entity.start,
        end: entity.end,
        text: entity.text,
        confidence: entity.confidence,
        note: 'context hint assembled from decoded ML entity span',
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

  const decoded = decodeReminderMlEntities(
    ctx.rawInput,
    encoding.inputIds,
    encoding.offsets,
    logits,
    artifact.idToLabel,
    artifact.tokenizer.specialTokenIds,
  );
  const entities = postprocessReminderMlEntities(ctx.rawInput, decoded);

  applyScheduleEntities(
    ctx,
    ctx.rawInput,
    opts,
    entities.filter((entity) =>
      ['recurrence', 'recurrence_start', 'recurrence_end', 'recurrence_exception', 'recurrence_window', 'lead_time'].includes(
        entity.label,
      ),
    ),
  );
  applyTriggerTimeEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'trigger_time'));
  applyLocationEntities(ctx, entities.filter((entity) => entity.label === 'location'));
  applyLeadTimeEntities(ctx, entities.filter((entity) => entity.label === 'lead_time'));

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
