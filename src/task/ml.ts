import { priorityPass } from './passes/priorityPass.ts';
import { applyCaptureSchedule } from './passes/schedulePass.ts';
import { statusPass } from './passes/statusPass.ts';
import { tagsPass } from './passes/tagsPass.ts';
import { titlePass } from './passes/titlePass.ts';
import { finalizeTaskParseContext } from './finalize.ts';
import type { TaskFields, TaskParseContext, TaskParseOptions, TaskParseResult } from './types.ts';
import { createTaskParseContext, extractAssignees } from './utils.ts';
import { extractProject } from './utils.ts';
import { captureSchedule } from '../dates/index.ts';
import type { InferenceAdapter, LoadedMlArtifact, MlArtifactSource, RuntimeTensor } from '../runtime/types.ts';

const PUNCT_TRIM = ' ,;:.';
const PUNCT_ONLY_REGEX = /^[,;:.]+$/;
const TIMEISH_REGEX = /^by\s+(?:\d{1,2}(?::\d{2})?\s*(?:am|pm)?|noon|midnight)\b/i;
const RECURRENCE_MONTH_TAIL_REGEX = /\bof every$/i;
const PRIORITY_LIKE_REGEX = /(?:priority|priorty|urgent|low|medium|high)/i;
const TAG_TOKEN_REGEX = /#?[A-Za-z0-9][A-Za-z0-9_-]*/g;
const ASSIGNEE_NOISE = new Set(['and', '&']);
const SCHEDULE_LABELS = new Set([
  'due_date',
  'due_time',
  'duration',
  'recurrence',
  'recurrence_start',
  'recurrence_end',
  'recurrence_exception',
]);

type ConfidenceField = keyof TaskParseResult['meta']['confidence'];
type SpanField = keyof TaskParseResult['meta']['spans'];

export interface TaskMlEntity {
  label: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
}

// ML mode is intentionally explicit: callers must provide both a verified artifact
// and an inference adapter via createTaskParser({ mode: 'ml', artifact, inference }).
export type TaskParserConfig =
  | { mode?: 'lightweight' }
  | {
      mode: 'ml';
      artifact: MlArtifactSource | LoadedMlArtifact<'task'>;
      inference: InferenceAdapter;
    };

export interface TaskParserInfo {
  parser: 'task';
  suite_version?: string;
  artifact_id?: string;
}

export interface TaskParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: TaskParserInfo;
  parse(input: string, opts?: TaskParseOptions): Promise<TaskParseResult>;
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
): TaskMlEntity => ({
  label: current.label,
  text: text.slice(current.start, current.end),
  start: current.start,
  end: current.end,
  confidence: Number((current.confidenceSum / Math.max(current.confidenceCount, 1)).toFixed(4)),
});

export const decodeTaskMlEntities = (
  text: string,
  inputIds: number[],
  offsets: Array<[number, number]>,
  logits: RuntimeTensor,
  idToLabel: Record<number, string>,
  specialTokenIds: Set<number>,
): TaskMlEntity[] => {
  const [batch, sequenceLength, labelCount] = logits.shape;
  if (batch !== 1) {
    throw new Error(`Task logits tensor must have batch size 1, received ${batch}`);
  }
  if (sequenceLength !== inputIds.length) {
    throw new Error(`Task logits sequence length mismatch: expected ${inputIds.length}, received ${sequenceLength}`);
  }
  if (!Number.isInteger(labelCount) || labelCount <= 0) {
    throw new Error('Task logits tensor is missing a valid label dimension');
  }

  const entities: TaskMlEntity[] = [];
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

const trimAndDropNoise = (entities: TaskMlEntity[]): TaskMlEntity[] => {
  const cleaned: TaskMlEntity[] = [];
  const sorted = [...entities].sort((left, right) => left.start - right.start || left.end - right.end);

  for (const entity of sorted) {
    let start = entity.start;
    let text = entity.text;
    const leadTrim = text.length - text.replace(new RegExp(`^[${PUNCT_TRIM.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&')}]+`), '').length;
    const tailTrim = text.length - text.replace(new RegExp(`[${PUNCT_TRIM.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&')}]+$`), '').length;
    if (leadTrim > 0) {
      start += leadTrim;
    }
    if (tailTrim > 0) {
      text = text.slice(0, text.length - tailTrim);
    }
    text = text.slice(leadTrim).trim();

    if (entity.label === 'assignee') {
      if (text.toLowerCase().startsWith('to ')) {
        start += 3;
        text = text.slice(3);
      }
      if (text.toLowerCase().startsWith('with ')) {
        start += 5;
        text = text.slice(5);
      }
    } else if (entity.label === 'project' && text.toLowerCase().startsWith('for ')) {
      start += 4;
      text = text.slice(4);
    }

    text = text.trim().replace(/^[,;:. ]+|[,;:. ]+$/g, '');
    if (!text) {
      continue;
    }
    if (entity.label === 'status' && PUNCT_ONLY_REGEX.test(text)) {
      continue;
    }
    if (entity.label === 'assignee' && ASSIGNEE_NOISE.has(text.toLowerCase())) {
      continue;
    }

    cleaned.push({
      ...entity,
      text,
      start,
      end: start + text.length,
    });
  }

  return cleaned;
};

const mergeDueTime = (entities: TaskMlEntity[]): TaskMlEntity[] => {
  const merged: TaskMlEntity[] = [];
  let index = 0;

  while (index < entities.length) {
    const entity = cloneValue(entities[index]);

    if (entity.label === 'due_date' && TIMEISH_REGEX.test(entity.text)) {
      entity.label = 'due_time';
      merged.push(entity);
      index += 1;
      continue;
    }

    if (entity.label === 'due_date' && entity.text.toLowerCase() === 'by' && index + 1 < entities.length) {
      const nextEntity = cloneValue(entities[index + 1]);
      const gap = nextEntity.start - entity.end;
      if (nextEntity.label === 'due_time' && gap <= 1) {
        nextEntity.start = entity.start;
        nextEntity.text = `${entity.text} ${nextEntity.text}`.trim();
        nextEntity.end = nextEntity.start + nextEntity.text.length;
        merged.push(nextEntity);
        index += 2;
        continue;
      }
    }

    merged.push(entity);
    index += 1;
  }

  return merged;
};

const mergeRecurrenceExceptionTail = (entities: TaskMlEntity[]): TaskMlEntity[] => {
  const merged: TaskMlEntity[] = [];
  let index = 0;

  while (index < entities.length) {
    const entity = cloneValue(entities[index]);
    if (
      entity.label === 'recurrence_exception' &&
      index + 1 < entities.length &&
      entities[index + 1].label === 'recurrence'
    ) {
      const nextEntity = entities[index + 1];
      const gap = nextEntity.start - entity.end;
      if (gap <= 1 && RECURRENCE_MONTH_TAIL_REGEX.test(entity.text) && nextEntity.text.toLowerCase() === 'month') {
        entity.text = `${entity.text} ${nextEntity.text}`.trim();
        entity.end = nextEntity.end;
        merged.push(entity);
        index += 2;
        continue;
      }
    }

    merged.push(entity);
    index += 1;
  }

  return merged;
};

const mergePriorityFragments = (entities: TaskMlEntity[]): TaskMlEntity[] => {
  const merged: TaskMlEntity[] = [];
  let index = 0;

  while (index < entities.length) {
    const entity = cloneValue(entities[index]);
    if (entity.label !== 'priority') {
      merged.push(entity);
      index += 1;
      continue;
    }

    let end = entity.end;
    const pieces = [entity.text];
    let confidenceTotal = entity.confidence;
    let confidenceCount = 1;
    let nextIndex = index + 1;

    while (nextIndex < entities.length) {
      const nextEntity = entities[nextIndex];
      if (!['priority', 'status'].includes(nextEntity.label)) {
        break;
      }
      const gap = nextEntity.start - end;
      if (gap !== 0) {
        break;
      }
      pieces.push(nextEntity.text);
      end = nextEntity.end;
      confidenceTotal += nextEntity.confidence;
      confidenceCount += 1;
      nextIndex += 1;
    }

    const combined = pieces.join('');
    if (confidenceCount > 1 && PRIORITY_LIKE_REGEX.test(combined)) {
      entity.text = combined;
      entity.end = end;
      entity.confidence = Number((confidenceTotal / confidenceCount).toFixed(4));
      merged.push(entity);
      index = nextIndex;
      continue;
    }

    merged.push(entity);
    index += 1;
  }

  return merged;
};

const dropStatusBeforePriority = (entities: TaskMlEntity[]): TaskMlEntity[] =>
  entities.filter((entity, index, source) => {
    if (
      entity.label === 'status' &&
      entity.text.toLowerCase() === 'with' &&
      index + 1 < source.length &&
      source[index + 1].label === 'priority' &&
      source[index + 1].start - entity.end <= 1
    ) {
      return false;
    }
    return true;
  });

const normalizeTags = (text: string, entities: TaskMlEntity[]): TaskMlEntity[] => {
  const normalized: TaskMlEntity[] = [];
  let index = 0;

  while (index < entities.length) {
    const entity = cloneValue(entities[index]);
    if (entity.label !== 'tags') {
      normalized.push(entity);
      index += 1;
      continue;
    }

    const start = entity.start;
    let end = entity.end;
    let combinedText = entity.text;
    let nextIndex = index + 1;

    while (nextIndex < entities.length && entities[nextIndex].label === 'tags') {
      const gap = entities[nextIndex].start - end;
      if (gap > 1) {
        break;
      }
      combinedText += text.slice(end, entities[nextIndex].start) + entities[nextIndex].text;
      end = entities[nextIndex].end;
      nextIndex += 1;
    }

    if (combinedText.includes('#')) {
      const sliceText = text.slice(start, end);
      for (const match of sliceText.matchAll(/#[A-Za-z0-9][A-Za-z0-9_-]*/g)) {
        const tagText = match[0] || '';
        if (!tagText) {
          continue;
        }
        normalized.push({
          label: 'tags',
          start: start + (match.index ?? 0),
          end: start + (match.index ?? 0) + tagText.length,
          text: tagText,
          confidence: entity.confidence,
        });
      }
    } else {
      TAG_TOKEN_REGEX.lastIndex = 0;
      for (const match of combinedText.matchAll(TAG_TOKEN_REGEX)) {
        const token = match[0] || '';
        if (!token) {
          continue;
        }
        normalized.push({
          label: 'tags',
          start: start + (match.index ?? 0),
          end: start + (match.index ?? 0) + token.length,
          text: token.startsWith('#') ? token : `#${token}`,
          confidence: entity.confidence,
        });
      }
    }

    index = nextIndex;
  }

  return normalized;
};

export const postprocessTaskMlEntities = (text: string, entities: TaskMlEntity[]): TaskMlEntity[] => {
  let cleaned = trimAndDropNoise(entities);
  cleaned = mergeDueTime(cleaned);
  cleaned = mergeRecurrenceExceptionTail(cleaned);
  cleaned = mergePriorityFragments(cleaned);
  cleaned = dropStatusBeforePriority(cleaned);
  cleaned = normalizeTags(text, cleaned);
  return cleaned;
};

const createMaskedEntityContext = (
  input: string,
  opts: TaskParseOptions | undefined,
  entities: TaskMlEntity[],
): TaskParseContext => {
  const ctx = createTaskParseContext(input, opts);
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

const appendDebugSteps = (ctx: TaskParseContext, steps: TaskParseResult['meta']['debugSteps']): void => {
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

const applyFieldValue = <TField extends keyof TaskFields>(
  ctx: TaskParseContext,
  field: TField,
  value: TaskFields[TField],
  confidence: number,
  spans: TaskParseResult['meta']['spans'][TField],
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

const mergeFieldFromContext = <TField extends keyof TaskFields>(
  ctx: TaskParseContext,
  sourceCtx: TaskParseContext,
  field: TField,
): void => {
  const updated = applyFieldValue(
    ctx,
    field,
    sourceCtx.result.fields[field],
    sourceCtx.result.meta.confidence[field as ConfidenceField],
    sourceCtx.result.meta.spans[field as SpanField] as TaskParseResult['meta']['spans'][TField],
  );
  if (updated) {
    appendDebugSteps(ctx, sourceCtx.result.meta.debugSteps);
  }
};

const maskTaskSpan = (ctx: TaskParseContext, start: number, end: number): void => {
  if (end <= start || start < 0 || end > ctx.maskedInput.length) {
    return;
  }

  const before = ctx.maskedInput.slice(0, start);
  const middle = ' '.repeat(end - start);
  const after = ctx.maskedInput.slice(end);
  ctx.maskedInput = `${before}${middle}${after}`;
  ctx.result.meta.maskedInput = ctx.maskedInput;
};

const entitySpan = (entity: TaskMlEntity) => ({
  start: entity.start,
  end: entity.end,
  text: entity.text,
});

const applyPriorityEntities = (ctx: TaskParseContext, input: string, opts: TaskParseOptions | undefined, entities: TaskMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }
  const subctx = createMaskedEntityContext(input, opts, entities);
  priorityPass(subctx);
  mergeFieldFromContext(ctx, subctx, 'priority');
};

const applyStatusEntities = (ctx: TaskParseContext, input: string, opts: TaskParseOptions | undefined, entities: TaskMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }
  const subctx = createMaskedEntityContext(input, opts, entities);
  statusPass(subctx);
  mergeFieldFromContext(ctx, subctx, 'status');
};

const applyTagsEntities = (ctx: TaskParseContext, input: string, opts: TaskParseOptions | undefined, entities: TaskMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }
  const subctx = createMaskedEntityContext(input, opts, entities);
  tagsPass(subctx);
  mergeFieldFromContext(ctx, subctx, 'tags');
};

const applyProjectEntities = (ctx: TaskParseContext, entities: TaskMlEntity[]): void => {
  const sorted = [...entities].sort((left, right) => right.confidence - left.confidence);
  for (const entity of sorted) {
    const extraction = extractProject(`for ${entity.text}`);
    if (!extraction.project) {
      continue;
    }

    const applied = applyFieldValue(ctx, 'project', extraction.project, entity.confidence, [entitySpan(entity)]);
    if (!applied) {
      continue;
    }

    appendDebugSteps(ctx, [
      {
        pass: 'ml',
        ruleId: 'ml.project_entity',
        start: entity.start,
        end: entity.end,
        text: entity.text,
        confidence: entity.confidence,
        note: 'project assembled from decoded ML entity span',
      },
    ]);
    return;
  }
};

const clusterEntities = (input: string, entities: TaskMlEntity[], joiner: RegExp): TaskMlEntity[] => {
  const sorted = [...entities].sort((left, right) => left.start - right.start || left.end - right.end);
  if (sorted.length === 0) {
    return [];
  }

  const clusters: TaskMlEntity[] = [];
  let current = cloneValue(sorted[0]);
  let confidenceTotal = current.confidence;
  let confidenceCount = 1;

  for (let index = 1; index < sorted.length; index += 1) {
    const next = sorted[index];
    const gapText = input.slice(current.end, next.start);
    if (joiner.test(gapText)) {
      current = {
        ...current,
        text: `${current.text}${gapText}${next.text}`,
        end: next.end,
      };
      confidenceTotal += next.confidence;
      confidenceCount += 1;
      continue;
    }

    clusters.push({
      ...current,
      confidence: Number((confidenceTotal / confidenceCount).toFixed(4)),
    });
    current = cloneValue(next);
    confidenceTotal = next.confidence;
    confidenceCount = 1;
  }

  clusters.push({
    ...current,
    confidence: Number((confidenceTotal / confidenceCount).toFixed(4)),
  });
  return clusters;
};

const applyAssigneeEntities = (ctx: TaskParseContext, entities: TaskMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const clusters = clusterEntities(ctx.rawInput, entities, /^\s*(?:,|and|&)\s*$/i);
  const assignees: string[] = [];
  const spans: Array<{ start: number; end: number; text: string }> = [];
  const seen = new Set<string>();
  let confidence = 0;

  for (const cluster of clusters) {
    const extraction = extractAssignees(`to ${cluster.text}`, ctx.options.knownAssignees);
    if (extraction.assignees.length === 0) {
      continue;
    }
    for (const value of extraction.assignees) {
      const key = value.toLowerCase();
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      assignees.push(value);
    }
    spans.push(entitySpan(cluster));
    confidence = Math.max(confidence, cluster.confidence);
  }

  const applied = applyFieldValue(ctx, 'assignee', assignees, confidence, spans);
  if (!applied) {
    return;
  }

  appendDebugSteps(
    ctx,
    spans.map((span) => ({
      pass: 'ml',
      ruleId: 'ml.assignee_entity',
      start: span.start,
      end: span.end,
      text: span.text,
      confidence,
      note: 'assignee assembled from decoded ML entity span',
    })),
  );
};

const applyScheduleEntities = (
  ctx: TaskParseContext,
  input: string,
  opts: TaskParseOptions | undefined,
  entities: TaskMlEntity[],
): void => {
  if (entities.length === 0) {
    return;
  }

  const subctx = createTaskParseContext(input, opts);
  const found = captureSchedule(subctx.rawInput, 'task', {
    now: subctx.now,
    timezone: subctx.options.timezone,
    locale: subctx.options.locale,
    spans: entities.map((entity) => ({ label: entity.label, start: entity.start, end: entity.end })),
  });
  applyCaptureSchedule(subctx, found, Math.max(...entities.map((entity) => entity.confidence)));
  mergeFieldFromContext(ctx, subctx, 'due_date');
  mergeFieldFromContext(ctx, subctx, 'due_time');
  mergeFieldFromContext(ctx, subctx, 'duration');
  mergeFieldFromContext(ctx, subctx, 'recurrence');
  mergeFieldFromContext(ctx, subctx, 'recurrence_start');
  mergeFieldFromContext(ctx, subctx, 'recurrence_end');
  mergeFieldFromContext(ctx, subctx, 'recurrence_exception');
  ctx.state.recurring = subctx.state.recurring;
  ctx.result.schedule = found.schedule;
  if (subctx.result.warnings) {
    ctx.result.warnings = [...(ctx.result.warnings || []), ...subctx.result.warnings];
  }
};

const applyTitleEntities = (ctx: TaskParseContext, input: string, opts: TaskParseOptions | undefined, entities: TaskMlEntity[]): void => {
  if (entities.length === 0) {
    return;
  }

  const subctx = createMaskedEntityContext(input, opts, entities);
  titlePass(subctx);
  subctx.result.meta.confidence.title = Math.max(subctx.result.meta.confidence.title, ...entities.map((entity) => entity.confidence));
  mergeFieldFromContext(ctx, subctx, 'title');
};

const validateLogitsTensor = (artifact: LoadedMlArtifact<'task'>, encodingLength: number, tensor: RuntimeTensor): void => {
  if (tensor.dtype !== 'float32') {
    throw new Error(`Task logits tensor must be float32, received ${tensor.dtype}`);
  }
  if (tensor.shape.length !== 3) {
    throw new Error(`Task logits tensor must be rank 3, received rank ${tensor.shape.length}`);
  }
  if (tensor.shape[0] !== 1) {
    throw new Error(`Task logits tensor must have batch size 1, received ${tensor.shape[0]}`);
  }
  if (tensor.shape[1] !== encodingLength) {
    throw new Error(`Task logits tensor sequence length mismatch: expected ${encodingLength}, received ${tensor.shape[1]}`);
  }
  if (tensor.shape[2] !== Object.keys(artifact.idToLabel).length) {
    throw new Error(`Task logits tensor label dimension mismatch: expected ${Object.keys(artifact.idToLabel).length}, received ${tensor.shape[2]}`);
  }
};

export const parseTaskInputWithMl = async (
  input: string,
  opts: TaskParseOptions | undefined,
  artifact: LoadedMlArtifact<'task'>,
  inference: InferenceAdapter,
): Promise<TaskParseResult> => {
  const ctx = createTaskParseContext(input, opts);
  ctx.result.meta.runtime = {
    mode: 'ml',
    suite_version: artifact.suite_version,
    artifact_id: artifact.artifact_id,
  };

  if (!ctx.rawInput) {
    titlePass(ctx);
    return finalizeTaskParseContext(ctx);
  }

  const encoding = artifact.tokenizer.encode(ctx.rawInput);
  const request = {
    parser: 'task' as const,
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
    throw new Error('Task inference adapter did not return logits');
  }
  validateLogitsTensor(artifact, encoding.inputIds.length, logits);

  const decoded = decodeTaskMlEntities(
    ctx.rawInput,
    encoding.inputIds,
    encoding.offsets,
    logits,
    artifact.idToLabel,
    artifact.tokenizer.specialTokenIds,
  );
  const entities = postprocessTaskMlEntities(ctx.rawInput, decoded);

  applyPriorityEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'priority'));
  applyStatusEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'status'));
  applyTagsEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'tags'));
  applyProjectEntities(ctx, entities.filter((entity) => entity.label === 'project'));
  applyAssigneeEntities(ctx, entities.filter((entity) => entity.label === 'assignee'));
  applyScheduleEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => SCHEDULE_LABELS.has(entity.label)));

  for (const entity of entities) {
    if (entity.label === 'title') {
      continue;
    }
    maskTaskSpan(ctx, entity.start, entity.end);
  }

  ctx.working = ctx.maskedInput;
  titlePass(ctx);
  if (ctx.result.fields.title === 'Task' && entities.some((entity) => entity.label === 'title')) {
    applyTitleEntities(ctx, ctx.rawInput, opts, entities.filter((entity) => entity.label === 'title'));
  } else if (entities.some((entity) => entity.label === 'title')) {
    ctx.result.meta.confidence.title = Math.max(
      ctx.result.meta.confidence.title,
      ...entities.filter((entity) => entity.label === 'title').map((entity) => entity.confidence),
    );
  }

  if (encoding.truncated) {
    ctx.result.warnings = [
      ...(ctx.result.warnings || []),
      {
        code: 'ml_input_truncated',
        severity: 'warning',
        message: 'Input exceeded the task ML maximum sequence length and was truncated.',
        fieldHints: ['title'],
        spans: [],
        details: {
          maxSequenceLength: artifact.manifest.tensor_contract.max_sequence_length,
        },
      },
    ];
  }

  return finalizeTaskParseContext(ctx);
};
