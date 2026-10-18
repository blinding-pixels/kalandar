import { captureSchedule } from '../dates/index.ts';
import { attendeesPass } from './passes/attendeesPass.ts';
import { applyCaptureSchedule } from './passes/schedulePass.ts';
import { titlePass } from './passes/titlePass.ts';
import { extractCalendarContactSuggestions } from './contactSuggestions.ts';
import { finalizeCalendarParseContext } from './finalize.ts';
import { classifyRecurrenceException } from './recurrenceExceptions.ts';
import type {
  DebugStep,
  EventFields,
  EventParseResult,
  FieldSpan,
  ParseContext,
  ParseOptions,
  StructuredRecurrenceException,
} from './types.ts';
import { createParseContext, maskSpan } from './utils.ts';
import type { InferenceAdapter, LoadedMlArtifact, MlArtifactSource, RuntimeTensor } from '../runtime/types.ts';

const RECURRENCE_START_CUE = /\b(?:starting|beginning)\s+(?:next|the following)\b/i;
const SCHEDULE_LABELS = new Set([
  'start_time',
  'end_time',
  'date',
  'time',
  'duration',
  'recurrence',
  'recurrence_start',
  'recurrence_end',
  'recurrence_exception',
  'lead_time',
  'alert',
]);

type ConfidenceField = keyof EventParseResult['meta']['confidence'];
type SpanField = keyof EventParseResult['meta']['spans'];

export interface CalendarMlEntity {
  label: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
  value?: StructuredRecurrenceException;
}

export type CalendarParserConfig =
  | { mode?: 'lightweight' }
  | {
      mode: 'ml';
      artifact: MlArtifactSource | LoadedMlArtifact<'calendar'>;
      inference: InferenceAdapter;
    };

export interface CalendarParserInfo {
  parser: 'calendar';
  suite_version?: string;
  artifact_id?: string;
}

export interface CalendarParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: CalendarParserInfo;
  parse(input: string, opts?: ParseOptions): Promise<EventParseResult>;
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
): CalendarMlEntity => ({
  label: current.label,
  text: text.slice(current.start, current.end),
  start: current.start,
  end: current.end,
  confidence: Number((current.confidenceSum / Math.max(current.confidenceCount, 1)).toFixed(4)),
});

const decodeCalendarEntities = (
  text: string,
  inputIds: number[],
  offsets: Array<[number, number]>,
  logits: RuntimeTensor,
  idToLabel: Record<number, string>,
  specialTokenIds: Set<number>,
): CalendarMlEntity[] => {
  const [batch, sequenceLength, labelCount] = logits.shape;
  if (batch !== 1) {
    throw new Error(`Calendar logits tensor must have batch size 1, received ${batch}`);
  }
  if (sequenceLength !== inputIds.length) {
    throw new Error(`Calendar logits sequence length mismatch: expected ${inputIds.length}, received ${sequenceLength}`);
  }
  if (!Number.isInteger(labelCount) || labelCount <= 0) {
    throw new Error('Calendar logits tensor is missing a valid label dimension');
  }

  const entities: CalendarMlEntity[] = [];
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

const trimEntity = (entity: CalendarMlEntity): CalendarMlEntity | null => {
  const trimmed = entity.text.replace(/^[\s,;:.]+|[\s,;:.]+$/g, '');
  if (!trimmed) {
    return null;
  }

  const leadingWhitespace = entity.text.match(/^[\s,;:.]*/)?.[0].length || 0;
  const nextStart = entity.start + leadingWhitespace;
  return {
    ...entity,
    text: trimmed,
    start: nextStart,
    end: nextStart + trimmed.length,
  };
};

export const postprocessCalendarMlEntities = (
  entities: CalendarMlEntity[],
  options?: ParseOptions,
): CalendarMlEntity[] => {
  const output: CalendarMlEntity[] = [];

  for (const rawEntity of entities) {
    const trimmed = trimEntity(rawEntity);
    if (!trimmed) {
      continue;
    }

    if (trimmed.label !== 'recurrence_exception') {
      output.push(trimmed);
      continue;
    }

    const cue = RECURRENCE_START_CUE.exec(trimmed.text);
    const finalText = cue && cue.index > 0 ? trimmed.text.slice(0, cue.index).replace(/[ ,;:.]+$/g, '') : trimmed.text;
    if (!finalText) {
      continue;
    }

    output.push({
      ...trimmed,
      text: finalText,
      end: trimmed.start + finalText.length,
      value: classifyRecurrenceException(finalText, {
        now: options?.now instanceof Date ? options.now : typeof options?.now === 'string' ? new Date(options.now) : undefined,
        timezone: options?.timezone,
      }),
    });
  }

  return output;
};

const createMaskedEntityContext = (
  input: string,
  opts: ParseOptions | undefined,
  entities: CalendarMlEntity[],
): ParseContext => {
  const ctx = createParseContext(input, opts);
  const maskedChars = ctx.rawInput.split('').map(() => ' ');

  for (const entity of entities) {
    for (let index = entity.start; index < entity.end && index < maskedChars.length; index += 1) {
      maskedChars[index] = ctx.rawInput[index];
    }
  }

  ctx.maskedInput = maskedChars.join('');
  ctx.result.meta.maskedInput = ctx.maskedInput;
  return ctx;
};

const appendDebugSteps = (ctx: ParseContext, steps: DebugStep[]): void => {
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
  return value !== null && value !== undefined;
};

const applyFieldValue = <TField extends keyof EventFields>(
  ctx: ParseContext,
  field: TField,
  value: EventFields[TField],
  confidence: number,
  spans: FieldSpan[],
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

const mergeFieldFromContext = <TField extends keyof EventFields>(
  ctx: ParseContext,
  sourceCtx: ParseContext,
  field: TField,
): void => {
  const updated = applyFieldValue(
    ctx,
    field,
    sourceCtx.result.fields[field],
    sourceCtx.result.meta.confidence[field as ConfidenceField],
    sourceCtx.result.meta.spans[field as SpanField],
  );
  if (updated) {
    appendDebugSteps(ctx, sourceCtx.result.meta.debugSteps);
  }
};

const normalizeLocationText = (text: string): string =>
  text
    .trim()
    .replace(/^(?:at|in|on|@)\s+/i, '')
    .replace(/^["']+|["']+$/g, '')
    .replace(/[ ,;:.]+$/g, '')
    .replace(/\s+/g, ' ')
    .trim();

const applyTitleEntities = (ctx: ParseContext, input: string, opts: ParseOptions | undefined, entities: CalendarMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const subctx = createMaskedEntityContext(input, opts, entities);
  titlePass(subctx);
  const entityConfidence = Math.max(...entities.map((entity) => entity.confidence), 0);
  subctx.result.meta.confidence.title = Math.max(subctx.result.meta.confidence.title, entityConfidence);
  for (const span of subctx.result.meta.spans.title) {
    span.confidence = Math.max(span.confidence, entityConfidence);
  }

  mergeFieldFromContext(ctx, subctx, 'title');
  mergeFieldFromContext(ctx, subctx, 'contact_suggestions');

  if (!ctx.result.fields.contact_suggestions && ctx.result.fields.title) {
    const suggestions = extractCalendarContactSuggestions(ctx.result.fields.title);
    if (suggestions.length > 0) {
      ctx.result.fields.contact_suggestions = suggestions;
      ctx.result.meta.confidence.contact_suggestions = 0.78;
    }
  }
};

const applyLocationEntities = (ctx: ParseContext, entities: CalendarMlEntity[]): void => {
  const sorted = [...entities].sort((left, right) => right.confidence - left.confidence);
  for (const entity of sorted) {
    const location = normalizeLocationText(entity.text);
    if (!location) {
      continue;
    }

    const applied = applyFieldValue(
      ctx,
      'location',
      location,
      entity.confidence,
      [
        {
          start: entity.start,
          end: entity.end,
          text: entity.text,
          ruleId: 'ml.location_entity',
          confidence: entity.confidence,
        },
      ],
    );
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

const applyScheduleEntities = (ctx: ParseContext, entities: CalendarMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const capture = captureSchedule(ctx.rawInput, 'calendar', {
    now: ctx.now,
    timezone: ctx.options.timezone,
    locale: ctx.options.locale,
    spans: entities.map(({ label, start, end }) => ({ label, start, end })),
  });
  const fallback = Math.max(...entities.map((entity) => entity.confidence));
  applyCaptureSchedule(ctx, capture, (span) => {
    const overlapping = entities.filter((entity) => entity.start < span.end && entity.end > span.start);
    return overlapping.length > 0 ? Math.max(...overlapping.map((entity) => entity.confidence)) : fallback;
  });
};

const validateLogitsTensor = (artifact: LoadedMlArtifact<'calendar'>, encodingLength: number, tensor: RuntimeTensor): void => {
  if (tensor.dtype !== 'float32') {
    throw new Error(`Calendar logits tensor must be float32, received ${tensor.dtype}`);
  }
  if (tensor.shape.length !== 3) {
    throw new Error(`Calendar logits tensor must be rank 3, received rank ${tensor.shape.length}`);
  }
  if (tensor.shape[0] !== 1) {
    throw new Error(`Calendar logits tensor must have batch size 1, received ${tensor.shape[0]}`);
  }
  if (tensor.shape[1] !== encodingLength) {
    throw new Error(`Calendar logits tensor sequence length mismatch: expected ${encodingLength}, received ${tensor.shape[1]}`);
  }
  if (tensor.shape[2] !== Object.keys(artifact.idToLabel).length) {
    throw new Error(
      `Calendar logits tensor label dimension mismatch: expected ${Object.keys(artifact.idToLabel).length}, received ${tensor.shape[2]}`,
    );
  }
};

export const parseCalendarInputWithMl = async (
  input: string,
  opts: ParseOptions | undefined,
  artifact: LoadedMlArtifact<'calendar'>,
  inference: InferenceAdapter,
): Promise<EventParseResult> => {
  const ctx = createParseContext(input, opts);
  ctx.result.meta.runtime = {
    mode: 'ml',
    suite_version: artifact.suite_version,
    artifact_id: artifact.artifact_id,
  };

  if (!ctx.rawInput) {
    return finalizeCalendarParseContext(ctx);
  }

  const encoding = artifact.tokenizer.encode(ctx.rawInput);
  const request = {
    parser: 'calendar' as const,
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
    throw new Error('Calendar inference adapter did not return logits');
  }
  validateLogitsTensor(artifact, encoding.inputIds.length, logits);

  const decoded = decodeCalendarEntities(
    ctx.rawInput,
    encoding.inputIds,
    encoding.offsets,
    logits,
    artifact.idToLabel,
    artifact.tokenizer.specialTokenIds,
  );
  const entities = postprocessCalendarMlEntities(decoded, opts);

  applyTitleEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'title'));
  applyScheduleEntities(ctx, entities.filter((entity) => SCHEDULE_LABELS.has(entity.label)));
  applyLocationEntities(ctx, entities.filter((entity) => entity.label === 'location'));

  for (const entity of entities) {
    maskSpan(ctx, entity.start, entity.end);
  }

  attendeesPass(ctx);

  if (encoding.truncated) {
    ctx.result.warnings = [
      ...(ctx.result.warnings || []),
      {
        code: 'ml_input_truncated',
        severity: 'warning',
        message: 'Input exceeded the calendar ML maximum sequence length and was truncated.',
        fieldHints: null,
        spans: null,
        details: {
          maxSequenceLength: artifact.manifest.tensor_contract.max_sequence_length,
        },
      },
    ];
  }

  return finalizeCalendarParseContext(ctx);
};
