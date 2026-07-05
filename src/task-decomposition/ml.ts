import type { InferenceAdapter, LoadedMlArtifact, RuntimeTensor } from '../runtime/types.ts';
import type {
  TaskDecompositionChildTask,
  TaskDecompositionField,
  TaskDecompositionParseOptions,
  TaskDecompositionPlanTask,
  TaskDecompositionResult,
} from './types.ts';

const POSITIVE_KINDS = new Set(['parallel', 'paired_targets', 'cross_product']);
const TITLE_MERGEABLE_LABELS = new Set(['parent_title', 'child_item', 'child_target', 'axis_subject']);
const ACTION_LABEL = 'axis_action';
const TIME_LABELS = new Set(['shared_due_time', 'child_due_time']);
const PUNCT_ONLY_RE = /^[,;:.]+$/;
const PAIR_COUNT_RE = /\b\d+\b/g;
const LEADING_VERB_RE = /^(?<verb>[a-z]+)(?:\s+(?<article>the|a|an))?\s+(?<body>.+)$/i;
const GO_THROUGH_RE = /^go through\s+(?<body>.+)$/i;
const PAIRED_ACTION_PREFIX_RE = /^\s*(?<prefix>.+?),\s*one for\b/i;
const STANDARD_CROSS_PRODUCT_ACTIONS_RE = /\bfor each song\s+learn the\s+(?<actions>.+?)[.!?]?\s*$/i;
const REFERENTIAL_CROSS_PRODUCT_ACTIONS_RE =
  /\bgo through\s+(?<actions>.+?)\s+for each of these\s+(?:tunes|songs)\b/i;
const UNSUPPORTED_FLOW_RE = /\b(if|otherwise|before|after that)\b/i;
const PRONOUN_AMBIGUITY_RE = /\b(them|it|those|these)\b/i;
const BRANCHING_LIST_RE = /\b(?:then|and)\s+for\b/i;
const MIN_SUGGESTION_CONFIDENCE = 0.7;
const TEMPLATE_VERBS = new Set(['learn', 'memorize', 'practice', 'sing', 'transcribe', 'map', 'review', 'study', 'draft', 'write']);

type TaskDecompositionMlEntity = {
  label: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
};

const cloneValue = <T>(value: T): T =>
  typeof globalThis.structuredClone === 'function'
    ? globalThis.structuredClone(value)
    : (JSON.parse(JSON.stringify(value)) as T);

const buildTensor = (shape: number[], dtype: RuntimeTensor['dtype'], data: number[]): RuntimeTensor => ({
  shape,
  dtype,
  data,
});

const cleanPhrase = (input: string): string => input.replace(/^[\s,;:.!?]+|[\s,;:.!?]+$/g, '').replace(/\s+/g, ' ').trim();

const sentenceCase = (input: string): string => {
  const cleaned = cleanPhrase(input);
  if (!cleaned) {
    return cleaned;
  }
  return `${cleaned[0].toUpperCase()}${cleaned.slice(1)}`;
};

const field = (state: TaskDecompositionField['state'], value: string | null): TaskDecompositionField => ({ state, value });

const explicitOrUnspecified = (value: string | null): TaskDecompositionField =>
  value ? field('explicit', value) : field('unspecified', null);

const inheritedOrUnspecified = (value: string | null): TaskDecompositionField =>
  value ? field('inherited', value) : field('unspecified', null);

const explicitOrInherited = (explicit: string | null, shared: string | null): TaskDecompositionField =>
  explicit ? field('explicit', explicit) : inheritedOrUnspecified(shared);

const buildParent = (title: string | null, sharedDueDate: string | null, sharedDueTime: string | null, sharedProject: string | null): TaskDecompositionPlanTask | null =>
  title
    ? {
        title: cleanPhrase(title),
        due_date: explicitOrUnspecified(sharedDueDate),
        due_time: explicitOrUnspecified(sharedDueTime),
        project: explicitOrUnspecified(sharedProject),
      }
    : null;

const buildChild = (input: {
  title: string;
  dueDate?: string | null;
  dueTime?: string | null;
  project?: string | null;
  sharedDueDate: string | null;
  sharedDueTime: string | null;
  sharedProject: string | null;
  target?: string | null;
  subject?: string | null;
  action?: string | null;
}): TaskDecompositionChildTask => ({
  title: sentenceCase(input.title),
  due_date: explicitOrInherited(input.dueDate || null, input.sharedDueDate),
  due_time: explicitOrInherited(input.dueTime || null, input.sharedDueTime),
  project: explicitOrInherited(input.project || null, input.sharedProject),
  target: input.target || null,
  subject: input.subject || null,
  action: input.action || null,
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
  current: { label: string; start: number; end: number; confidenceSum: number; confidenceCount: number },
): TaskDecompositionMlEntity => ({
  label: current.label,
  text: text.slice(current.start, current.end),
  start: current.start,
  end: current.end,
  confidence: Number((current.confidenceSum / Math.max(current.confidenceCount, 1)).toFixed(4)),
});

export const decodeTaskDecompositionMlEntities = (
  text: string,
  inputIds: number[],
  offsets: Array<[number, number]>,
  logits: RuntimeTensor,
  idToLabel: Record<number, string>,
  specialTokenIds: Set<number>,
): TaskDecompositionMlEntity[] => {
  const [batch, sequenceLength, labelCount] = logits.shape;
  if (batch !== 1) {
    throw new Error(`Task decomposition logits tensor must have batch size 1, received ${batch}`);
  }
  if (sequenceLength !== inputIds.length) {
    throw new Error(`Task decomposition logits sequence length mismatch: expected ${inputIds.length}, received ${sequenceLength}`);
  }
  if (!Number.isInteger(labelCount) || labelCount <= 0) {
    throw new Error('Task decomposition logits tensor is missing a valid label dimension');
  }

  const entities: TaskDecompositionMlEntity[] = [];
  let current:
    | { label: string; start: number; end: number; confidenceSum: number; confidenceCount: number }
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
      current = { label, start, end, confidenceSum: confidence, confidenceCount: 1 };
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

const stripEntity = (entity: TaskDecompositionMlEntity): TaskDecompositionMlEntity | null => {
  const leading = entity.text.length - entity.text.replace(/^[ ,;:.]+/, '').length;
  const trailing = entity.text.length - entity.text.replace(/[ ,;:.]+$/, '').length;
  const start = entity.start + leading;
  const end = entity.end - trailing;
  if (end <= start) {
    return null;
  }
  const text = cleanPhrase(entity.text.slice(leading, trailing ? entity.text.length - trailing : entity.text.length));
  if (!text) {
    return null;
  }
  return { ...entity, text, start, end: start + text.length };
};

const canMergeEntities = (left: TaskDecompositionMlEntity, right: TaskDecompositionMlEntity, gap: string): boolean => {
  if (left.label !== right.label) {
    return false;
  }
  if (TITLE_MERGEABLE_LABELS.has(left.label) && ['', ' ', '  '].includes(gap)) {
    return true;
  }
  if (left.label === ACTION_LABEL && ['', ' ', ' the ', ' a ', ' an '].includes(gap.toLowerCase())) {
    return true;
  }
  return false;
};

const shouldDropEntity = (entity: TaskDecompositionMlEntity): boolean => {
  if (TIME_LABELS.has(entity.label) && PUNCT_ONLY_RE.test(entity.text)) {
    return true;
  }
  if (entity.text.length === 1 && ['shared_due_time', 'shared_due_date'].includes(entity.label)) {
    return true;
  }
  return false;
};

const normalizeEntities = (text: string, entities: TaskDecompositionMlEntity[]): TaskDecompositionMlEntity[] => {
  const cleaned = entities
    .map((entity) => stripEntity(entity))
    .filter((entity): entity is TaskDecompositionMlEntity => Boolean(entity))
    .filter((entity) => !PUNCT_ONLY_RE.test(entity.text))
    .sort((left, right) => left.start - right.start || left.end - right.end);

  const merged: TaskDecompositionMlEntity[] = [];
  for (const entity of cleaned) {
    const previous = merged[merged.length - 1];
    if (previous) {
      const gap = text.slice(previous.end, entity.start);
      if (canMergeEntities(previous, entity, gap)) {
        merged[merged.length - 1] = {
          label: previous.label,
          text: cleanPhrase(text.slice(previous.start, entity.end)),
          start: previous.start,
          end: entity.end,
          confidence: Number(((previous.confidence + entity.confidence) / 2).toFixed(4)),
        };
        continue;
      }
    }
    merged.push(entity);
  }

  return merged.filter((entity) => !shouldDropEntity(entity));
};

const firstText = (entities: TaskDecompositionMlEntity[]): string | null => entities[0]?.text || null;

const chooseSlotText = (entities: TaskDecompositionMlEntity[], preferLongest = false): string | null => {
  if (entities.length === 0) {
    return null;
  }
  const candidates = entities.map((entity) => entity.text).filter((text) => !['by', 'at', 'on'].includes(text.toLowerCase()));
  const values = candidates.length > 0 ? candidates : entities.map((entity) => entity.text);
  return preferLongest ? values.sort((left, right) => right.length - left.length)[0] : values[0];
};

const entityTexts = (entities: TaskDecompositionMlEntity[]): string[] => entities.map((entity) => entity.text);

const chooseParentTitle = (text: string, entities: TaskDecompositionMlEntity[]): string | null => {
  if (entities.length === 0) {
    return null;
  }
  const first = entities[0];
  let merged = cleanPhrase(entities.map((entity) => entity.text).join(' '));
  const prefix = text.slice(0, first.start).trim();
  const suffix = text.slice(first.end);
  if (prefix && /^[:\s]/.test(suffix) && /^[A-Za-z][A-Za-z ]*$/.test(prefix)) {
    merged = cleanPhrase(`${prefix} ${merged}`);
  }
  return merged || null;
};

const maxParenthesisDepth = (text: string): number => {
  let depth = 0;
  let maximum = 0;
  for (const character of text) {
    if (character === '(') {
      depth += 1;
      maximum = Math.max(maximum, depth);
    } else if (character === ')' && depth > 0) {
      depth -= 1;
    }
  }
  return maximum;
};

const splitListClause = (clause: string): string[] =>
  cleanPhrase(clause)
    .split(/,\s+and\s+|\s+and\s+|,\s*/i)
    .map((part) => cleanPhrase(part))
    .filter(Boolean);

const singularize = (token: string): string => {
  if (token.toLowerCase().endsWith('ies') && token.length > 3) {
    return `${token.slice(0, -3)}y`;
  }
  if (token.toLowerCase().endsWith('ses') && token.length > 3) {
    return token.slice(0, -2);
  }
  if (token.toLowerCase().endsWith('s') && !token.toLowerCase().endsWith('ss') && token.length > 2) {
    return token.slice(0, -1);
  }
  return token;
};

const recoverPairedParentActionFromText = (text: string): string | null => {
  const match = text.match(PAIRED_ACTION_PREFIX_RE);
  return match?.groups?.prefix ? cleanPhrase(match.groups.prefix) : null;
};

const normalizePairedParentAction = (parentTitle: string | null, text: string): string => {
  const recovered = recoverPairedParentActionFromText(text);
  const title = recovered || parentTitle || '';
  if (!title) {
    return '';
  }
  const action = cleanPhrase(title.replace(PAIR_COUNT_RE, ''));
  const parts = action.split(/\s+/).filter(Boolean);
  if (parts.length === 0) {
    return action;
  }
  parts[parts.length - 1] = singularize(parts[parts.length - 1]);
  return parts.join(' ');
};

const normalizeAxisActions = (text: string, actions: string[]): string[] => {
  if (actions.length === 0) {
    return actions;
  }
  const normalized: string[] = [];
  let templateVerb = '';
  let templateArticle = '';
  let templatePrefix = '';
  for (const action of actions) {
    const cleaned = cleanPhrase(action);
    const goThroughMatch = cleaned.match(GO_THROUGH_RE);
    if (goThroughMatch) {
      templatePrefix = 'go through';
      normalized.push(cleaned);
      continue;
    }
    const match = cleaned.match(LEADING_VERB_RE);
    if (match?.groups?.verb && TEMPLATE_VERBS.has(match.groups.verb.toLowerCase())) {
      templateVerb = match.groups.verb;
      templateArticle = match.groups.article || '';
      normalized.push(cleaned);
      continue;
    }
    if (templatePrefix) {
      normalized.push(cleanPhrase(`${templatePrefix} ${cleaned}`));
      continue;
    }
    if (templateVerb) {
      normalized.push(cleanPhrase(`${templateVerb} ${templateArticle ? `${templateArticle} ` : ''}${cleaned}`));
      continue;
    }
    normalized.push(cleaned);
  }

  const lowered = text.toLowerCase();
  if (templatePrefix || templateVerb) {
    return normalized;
  }
  if (lowered.includes('go through')) {
    return normalized.map((action) => cleanPhrase(`go through ${action}`));
  }
  if (lowered.includes('learn the')) {
    return normalized.map((action) => cleanPhrase(`learn the ${action}`));
  }
  return normalized;
};

const recoverAxisActionTexts = (text: string, actions: string[]): string[] => {
  const standardMatch = text.match(STANDARD_CROSS_PRODUCT_ACTIONS_RE);
  if (standardMatch?.groups?.actions) {
    return splitListClause(standardMatch.groups.actions).map((item) => `learn the ${item}`);
  }
  const referentialMatch = text.match(REFERENTIAL_CROSS_PRODUCT_ACTIONS_RE);
  if (referentialMatch?.groups?.actions) {
    return splitListClause(referentialMatch.groups.actions).map((item) => `go through ${item}`);
  }
  return normalizeAxisActions(text, actions);
};

const recoverSubjectFromText = (text: string, start: number, end: number, previousEnd: number): string => {
  let itemStart = start;
  const lowerBound = Math.max(previousEnd, 0);
  while (itemStart > lowerBound) {
    const previousCharacter = text[itemStart - 1];
    if (previousCharacter === ':' || previousCharacter === ';' || previousCharacter === ',') {
      break;
    }
    itemStart -= 1;
  }
  const candidate = cleanPhrase(text.slice(itemStart, end).replace(/^and\s+/i, ''));
  return candidate || text.slice(start, end);
};

const recoverAxisSubjectTexts = (text: string, entities: TaskDecompositionMlEntity[]): string[] => {
  if (entities.length < 2) {
    return entities.map((entity) => entity.text);
  }
  return entities.map((entity, index) => recoverSubjectFromText(text, entity.start, entity.end, index > 0 ? entities[index - 1].end : 0));
};

const allowsSupportedReference = (text: string, entities: TaskDecompositionMlEntity[], kind: string): boolean => {
  if (kind !== 'cross_product') {
    return false;
  }
  const axisSubjectCount = entities.filter((entity) => entity.label === 'axis_subject').length;
  const axisActionCount = entities.filter((entity) => entity.label === 'axis_action').length;
  if (axisSubjectCount < 2 || axisActionCount < 2) {
    return false;
  }
  const lowered = text.toLowerCase();
  return lowered.includes('for each of these') || lowered.includes('these tunes') || lowered.includes('these songs');
};

const hasFragmentedTitles = (children: TaskDecompositionChildTask[]): boolean =>
  children.some((child) => {
    const words = child.title.split(/\s+/).filter(Boolean);
    return words.some((word) => word.length === 1 && /[A-Za-z]/.test(word)) || (words.length <= 2 && child.title.length <= 4);
  });

const scoreSuggestion = (confidence: number, penalties: Array<[string, number]>): number =>
  Math.max(0, Math.min(1, confidence - penalties.reduce((total, [, weight]) => total + weight, 0)));

const v1HardBlockers = (text: string, kind: string): string[] => {
  if (!POSITIVE_KINDS.has(kind)) {
    return [];
  }
  return maxParenthesisDepth(text) > 1 ? ['nested_hierarchy_not_supported'] : [];
};

const v1SoftPenalties = (
  text: string,
  entities: TaskDecompositionMlEntity[],
  kind: string,
  confidence: number,
  children: TaskDecompositionChildTask[],
): Array<[string, number]> => {
  if (!POSITIVE_KINDS.has(kind)) {
    return [];
  }
  const penalties: Array<[string, number]> = [];
  if (confidence < MIN_SUGGESTION_CONFIDENCE) {
    penalties.push(['low_confidence', Math.min(0.35, (MIN_SUGGESTION_CONFIDENCE - confidence) * 1.5 + 0.1)]);
  } else if (confidence < 0.82) {
    penalties.push(['borderline_confidence', Math.min(0.12, (0.82 - confidence) * 1.0)]);
  }
  if ((text.match(/:/g) || []).length > 1) {
    penalties.push(['multiple_parent_heads_signal', 0.3]);
  }
  const flowMatch = text.match(UNSUPPORTED_FLOW_RE);
  if (flowMatch) {
    penalties.push(['flow_control_signal', ['if', 'otherwise'].includes(flowMatch[1].toLowerCase()) ? 0.28 : 0.16]);
  }
  if (PRONOUN_AMBIGUITY_RE.test(text) && kind !== 'paired_targets' && !allowsSupportedReference(text, entities, kind)) {
    penalties.push(['coreference_signal', 0.18]);
  }
  const parentSpanCount = entities.filter((entity) => entity.label === 'parent_title').length;
  if (parentSpanCount > 1) {
    penalties.push(['multiple_parent_spans_signal', Math.min(0.24, 0.12 * (parentSpanCount - 1))]);
  }
  if (kind === 'parallel' && text.includes('(') && text.includes(')')) {
    penalties.push(['single_nested_branch_signal', 0.18]);
  }
  if (text.includes('(') && text.includes(')') && BRANCHING_LIST_RE.test(text)) {
    penalties.push(['branching_nested_signal', 0.32]);
  }
  if (hasFragmentedTitles(children)) {
    penalties.push(['fragmented_child_signal', 0.12]);
  }
  return penalties;
};

const planFromEntities = (text: string, decodedEntities: TaskDecompositionMlEntity[]): Omit<TaskDecompositionResult, 'meta'> => {
  const normalized = normalizeEntities(text, decodedEntities);
  const byLabel = new Map<string, TaskDecompositionMlEntity[]>();
  for (const entity of normalized) {
    const bucket = byLabel.get(entity.label) || [];
    bucket.push(entity);
    byLabel.set(entity.label, bucket);
  }

  let parentTitle = chooseParentTitle(text, byLabel.get('parent_title') || []);
  const sharedDueDate = chooseSlotText(byLabel.get('shared_due_date') || [], true);
  const sharedDueTime = firstText(byLabel.get('shared_due_time') || []);
  const sharedProject = firstText(byLabel.get('shared_project') || []);
  const childItems = entityTexts(byLabel.get('child_item') || []);
  const childTargets = entityTexts(byLabel.get('child_target') || []);
  const axisSubjects = recoverAxisSubjectTexts(text, byLabel.get('axis_subject') || []);
  const axisActions = recoverAxisActionTexts(text, entityTexts(byLabel.get('axis_action') || []));
  const childDueDates = entityTexts(byLabel.get('child_due_date') || []);
  const childDueTimes = entityTexts(byLabel.get('child_due_time') || []);

  let kind: TaskDecompositionResult['kind'] = 'none';
  const issues: string[] = [];
  let children: TaskDecompositionChildTask[] = [];

  if (childItems.length >= 2) {
    kind = 'parallel';
    children = childItems.map((item) =>
      buildChild({ title: item, sharedDueDate, sharedDueTime, sharedProject }),
    );
  } else if (childTargets.length >= 2) {
    kind = 'paired_targets';
    const recoveredParent = recoverPairedParentActionFromText(text);
    if (recoveredParent) {
      parentTitle = recoveredParent;
    }
    const action = normalizePairedParentAction(parentTitle, text);
    children = childTargets.map((target, index) =>
      buildChild({
        title: action ? `${action} for ${target}` : target,
        target,
        dueDate: childDueDates[index] || null,
        dueTime: childDueTimes[index] || null,
        sharedDueDate,
        sharedDueTime,
        sharedProject,
      }),
    );
  } else if (axisSubjects.length >= 2 && axisActions.length >= 2) {
    kind = 'cross_product';
    children = axisSubjects.flatMap((subject) =>
      axisActions.map((action) =>
        buildChild({
          title: `${action} for ${subject}`,
          subject,
          action,
          sharedDueDate,
          sharedDueTime,
          sharedProject,
        }),
      ),
    );
  } else {
    issues.push('insufficient_decomposition_signals');
  }

  const confidenceEntities = kind === 'none' ? normalized : normalized.filter((entity) => entity.label !== 'parent_title');
  const confidenceSource = confidenceEntities.length > 0 ? confidenceEntities : normalized;
  const confidence =
    confidenceSource.reduce((total, entity) => total + entity.confidence, 0) / Math.max(confidenceSource.length, 1);

  const hardBlockers = v1HardBlockers(text, kind);
  const softPenalties = v1SoftPenalties(text, normalized, kind, confidence, children);
  const decisionScore = scoreSuggestion(confidence, softPenalties);
  issues.push(...hardBlockers, ...softPenalties.map(([issue]) => issue));

  const shouldSuggest = POSITIVE_KINDS.has(kind) && children.length >= 2 && hardBlockers.length === 0 && decisionScore >= MIN_SUGGESTION_CONFIDENCE;
  if (!shouldSuggest) {
    if (decisionScore < MIN_SUGGESTION_CONFIDENCE) {
      issues.push('decision_score_below_threshold');
    }
    kind = 'none';
    children = [];
  }

  return {
    should_suggest: shouldSuggest,
    kind,
    confidence: Number(confidence.toFixed(4)),
    parent: buildParent(parentTitle, sharedDueDate, sharedDueTime, sharedProject),
    children,
    issues: [...new Set(issues)].sort(),
  };
};

const validateLogitsTensor = (
  artifact: LoadedMlArtifact<'task_decomposition'>,
  encodingLength: number,
  tensor: RuntimeTensor,
): void => {
  if (tensor.dtype !== 'float32') {
    throw new Error(`Task decomposition logits tensor must be float32, received ${tensor.dtype}`);
  }
  if (tensor.shape.length !== 3) {
    throw new Error(`Task decomposition logits tensor must be rank 3, received rank ${tensor.shape.length}`);
  }
  if (tensor.shape[0] !== 1) {
    throw new Error(`Task decomposition logits tensor must have batch size 1, received ${tensor.shape[0]}`);
  }
  if (tensor.shape[1] !== encodingLength) {
    throw new Error(`Task decomposition logits tensor sequence length mismatch: expected ${encodingLength}, received ${tensor.shape[1]}`);
  }
  if (tensor.shape[2] !== Object.keys(artifact.idToLabel).length) {
    throw new Error(`Task decomposition logits tensor label dimension mismatch: expected ${Object.keys(artifact.idToLabel).length}, received ${tensor.shape[2]}`);
  }
};

export const parseTaskDecompositionInputWithMl = async (
  input: string,
  _opts: TaskDecompositionParseOptions | undefined,
  artifact: LoadedMlArtifact<'task_decomposition'>,
  inference: InferenceAdapter,
): Promise<TaskDecompositionResult> => {
  const rawInput = cleanPhrase(input || '');
  if (!rawInput) {
    return {
      should_suggest: false,
      kind: 'none',
      confidence: 0,
      parent: null,
      children: [],
      issues: ['empty_input'],
      meta: {
        parser: 'task_decomposition',
        runtime: {
          mode: 'ml',
          suite_version: artifact.suite_version,
          artifact_id: artifact.artifact_id,
        },
      },
    };
  }

  const encoding = artifact.tokenizer.encode(rawInput);
  const request = {
    parser: 'task_decomposition' as const,
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
    throw new Error('Task decomposition inference adapter did not return logits');
  }
  validateLogitsTensor(artifact, encoding.inputIds.length, logits);

  const decoded = decodeTaskDecompositionMlEntities(
    rawInput,
    encoding.inputIds,
    encoding.offsets,
    logits,
    artifact.idToLabel,
    artifact.tokenizer.specialTokenIds,
  );
  const result = planFromEntities(rawInput, decoded);
  return {
    ...cloneValue(result),
    meta: {
      parser: 'task_decomposition',
      runtime: {
        mode: 'ml',
        suite_version: artifact.suite_version,
        artifact_id: artifact.artifact_id,
      },
    },
  };
};
