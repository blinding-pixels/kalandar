import { parseTaskInput } from '../task/index.ts';
import type { TaskParseResult } from '../task/types.ts';
import { parseTaskDecompositionInputWithMl } from './ml.ts';
import type {
  TaskDecompositionChildTask,
  TaskDecompositionField,
  TaskDecompositionKind,
  TaskDecompositionParseOptions,
  TaskDecompositionParserConfig,
  TaskDecompositionParser,
  TaskDecompositionResult,
  TaskDecompositionPlanTask,
} from './types.ts';
import { loadMlArtifact } from '../runtime/index.ts';
import type { LoadedMlArtifact } from '../runtime/types.ts';

type SharedFields = {
  due_date: string | null;
  due_time: string | null;
  project: string | null;
};

type ChildSeed = {
  title: string;
  due_date?: string | null;
  due_time?: string | null;
  target?: string | null;
  subject?: string | null;
  action?: string | null;
};

const POSITIVE_KINDS = new Set<TaskDecompositionKind>(['parallel', 'paired_targets', 'cross_product']);
const META = { parser: 'task_decomposition' as const, runtime: { mode: 'lightweight' as const } };
const LEADING_AND_REGEX = /^(?:and|&)\s+/i;
const TRAILING_PUNCTUATION_REGEX = /[\s,;:.!?]+$/;
const PROJECT_PHRASE_SOURCE =
  "\\b(?:in|under)\\s+(?:the\\s+)?((?:project\\s+[A-Z][A-Za-z0-9-]*(?:\\s+[A-Z][A-Za-z0-9-]*){0,3})|(?:[A-Za-z][A-Za-z0-9'&-]*(?:\\s+[A-Za-z][A-Za-z0-9'&-]*){0,3}\\s+project))\\b|\\bfor\\s+((?:project\\s+[A-Z][A-Za-z0-9-]*(?:\\s+[A-Z][A-Za-z0-9-]*){0,3}))\\b";
const PROJECT_PHRASE_REGEX = new RegExp(PROJECT_PHRASE_SOURCE, 'gi');

const normalizeWhitespace = (input: string): string => input.replace(/\s+/g, ' ').trim();

const unique = (values: string[]): string[] => [...new Set(values.filter(Boolean))];

const cleanPhrase = (input: string): string =>
  normalizeWhitespace(input.replace(/^[\s,;:.!?]+/, '').replace(TRAILING_PUNCTUATION_REGEX, ''));

const sentenceCase = (input: string): string => {
  const cleaned = cleanPhrase(input);
  if (!cleaned) {
    return cleaned;
  }
  return `${cleaned[0].toUpperCase()}${cleaned.slice(1)}`;
};

const field = (state: TaskDecompositionField['state'], value: string | null): TaskDecompositionField => ({
  state,
  value,
});

const normalizeDueTimeValue = (value: string | null | undefined): string | null => {
  const cleaned = cleanPhrase(value || '');
  if (!cleaned) {
    return null;
  }
  return cleanPhrase(cleaned.replace(/^(?:at|by|before)\s+/i, ''));
};

const explicitOrUnspecified = (value: string | null): TaskDecompositionField =>
  value ? field('explicit', value) : field('unspecified', null);

const explicitDueTimeOrUnspecified = (value: string | null): TaskDecompositionField => {
  const normalized = normalizeDueTimeValue(value);
  return normalized ? field('explicit', normalized) : field('unspecified', null);
};

const inheritedOrUnspecified = (value: string | null): TaskDecompositionField =>
  value ? field('inherited', value) : field('unspecified', null);

const inheritedDueTimeOrUnspecified = (value: string | null): TaskDecompositionField => {
  const normalized = normalizeDueTimeValue(value);
  return normalized ? field('inherited', normalized) : field('unspecified', null);
};

const explicitOrInherited = (explicit: string | null | undefined, shared: string | null): TaskDecompositionField => {
  if (explicit) {
    return field('explicit', explicit);
  }
  return inheritedOrUnspecified(shared);
};

const explicitDueTimeOrInherited = (explicit: string | null | undefined, shared: string | null): TaskDecompositionField => {
  const explicitValue = normalizeDueTimeValue(explicit);
  if (explicitValue) {
    return field('explicit', explicitValue);
  }
  return inheritedDueTimeOrUnspecified(shared);
};

const emptyResult = (
  issues: string[] = [],
  parentTitle: string | null = null,
  shared: SharedFields = { due_date: null, due_time: null, project: null },
): TaskDecompositionResult => ({
  should_suggest: false,
  kind: 'none',
  confidence: parentTitle ? 0.35 : 0,
  parent: parentTitle ? buildParent(parentTitle, shared) : null,
  children: [],
  issues: unique(issues),
  meta: META,
});

const buildParent = (title: string, shared: SharedFields): TaskDecompositionPlanTask => ({
  title: cleanPhrase(title),
  due_date: explicitOrUnspecified(shared.due_date),
  due_time: explicitDueTimeOrUnspecified(shared.due_time),
  project: explicitOrUnspecified(shared.project),
});

const buildChildren = (seeds: ChildSeed[], shared: SharedFields): TaskDecompositionChildTask[] =>
  seeds.map((seed) => ({
    title: sentenceCase(seed.title),
    due_date: explicitOrInherited(seed.due_date, shared.due_date),
    due_time: explicitDueTimeOrInherited(seed.due_time, shared.due_time),
    project: inheritedOrUnspecified(shared.project),
    target: seed.target || null,
    subject: seed.subject || null,
    action: seed.action || null,
  }));

const buildResult = (
  kind: Exclude<TaskDecompositionKind, 'none'>,
  confidence: number,
  parentTitle: string,
  shared: SharedFields,
  childSeeds: ChildSeed[],
  issues: string[] = [],
): TaskDecompositionResult => {
  const children = buildChildren(childSeeds, shared).filter((child) => child.title.length > 0);
  const shouldSuggest = POSITIVE_KINDS.has(kind) && children.length >= 2 && cleanPhrase(parentTitle).length > 0;

  if (!shouldSuggest) {
    return emptyResult(['insufficient_decomposition_signals', ...issues]);
  }

  return {
    should_suggest: true,
    kind,
    confidence,
    parent: buildParent(parentTitle, shared),
    children,
    issues: unique(issues),
    meta: META,
  };
};

const escapeRegExp = (input: string): string => input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

const removeKnownFieldText = (input: string, value: string | null): string => {
  if (!value) {
    return input;
  }
  return normalizeWhitespace(input.replace(new RegExp(`\\s*(?:\\b(?:at|on|@)\\s+)?${escapeRegExp(value)}\\b`, 'i'), ' '));
};

const normalizeProjectValue = (input: string): string => {
  return cleanPhrase(input);
};

const extractSharedProject = (input: string): { text: string; project: string | null; start: number | null } => {
  let match: RegExpExecArray | null = null;
  for (const candidate of input.matchAll(PROJECT_PHRASE_REGEX)) {
    const start = candidate.index ?? 0;
    if (/\bone\s+$/i.test(input.slice(0, start))) {
      continue;
    }
    match = candidate;
    break;
  }
  PROJECT_PHRASE_REGEX.lastIndex = 0;
  if (!match || match.index == null) {
    return { text: input, project: null, start: null };
  }

  const text = normalizeWhitespace(`${input.slice(0, match.index)} ${input.slice(match.index + match[0].length)}`);
  return {
    text,
    project: normalizeProjectValue(match[0] || match[1] || match[2] || ''),
    start: match.index,
  };
};

const sharedFromTask = (rawInput: string, task: TaskParseResult): SharedFields => ({
  due_date: task.fields.due_date,
  due_time: task.fields.due_time,
  project: extractSharedProject(rawInput).project || task.fields.project,
});

const recoverAbstentionParentTitle = (rawInput: string, task: TaskParseResult): string | null => {
  const text = cleanPhrase(rawInput);
  if (!text) {
    return null;
  }

  const colonIndex = text.indexOf(':');
  if (colonIndex > 0) {
    return cleanPhrase(text.slice(0, colonIndex));
  }

  const orderedMatch = text.match(/^(?<parent>.+?)\s+and\s+then\b/i);
  if (orderedMatch?.groups?.parent) {
    return cleanPhrase(orderedMatch.groups.parent);
  }

  const selectiveMatch = text.match(/^(?<parent>.+?)[,]?\s+but\s+only\b/i);
  if (selectiveMatch?.groups?.parent) {
    return cleanPhrase(selectiveMatch.groups.parent);
  }

  const taskTitle = cleanPhrase(task.fields.title === 'Task' ? '' : task.fields.title).replace(/^just\s+/i, '');
  return cleanPhrase(taskTitle) || null;
};

const stripSharedFields = (rawInput: string, task: TaskParseResult): { text: string; shared: SharedFields } => {
  let text = rawInput;
  text = removeKnownFieldText(text, task.fields.due_time);
  text = removeKnownFieldText(text, task.fields.due_date);

  const projectExtraction = extractSharedProject(text);
  text = projectExtraction.text;

  return {
    text: cleanPhrase(text),
    shared: {
      due_date: task.fields.due_date,
      due_time: task.fields.due_time,
      project: projectExtraction.project,
    },
  };
};

const splitCommaList = (input: string): string[] =>
  input
    .split(',')
    .reduce<string[]>((parts, rawPart) => {
      const part = cleanPhrase(cleanPhrase(rawPart).replace(LEADING_AND_REGEX, ''));
      if (!part || /^(?:all|both)$/i.test(part)) {
        return parts;
      }
      if (/^as\s+in\b/i.test(part) && parts.length > 0) {
        parts[parts.length - 1] = `${parts[parts.length - 1]}, ${part}`;
        return parts;
      }
      parts.push(part);
      return parts;
    }, []);

const splitList = (input: string): string[] => splitCommaList(input);

const maxParenthesisDepth = (input: string): number => {
  let depth = 0;
  let maxDepth = 0;
  for (const char of input) {
    if (char === '(') {
      depth += 1;
      maxDepth = Math.max(maxDepth, depth);
    } else if (char === ')' && depth > 0) {
      depth -= 1;
    }
  }
  return maxDepth;
};

const hasNestedBranch = (input: string): boolean => {
  return /[()]/.test(input) || maxParenthesisDepth(input) > 0;
};

const findHardBlockers = (input: string): string[] => {
  const issues: string[] = [];
  if (/\b(?:if|otherwise)\b/i.test(input)) {
    issues.push('flow_control_signal');
  }
  if (/\b(?:before|after)\s+that\b/i.test(input)) {
    issues.push('ordered_dependency_signal');
  }
  if ((input.match(/:/g) || []).length > 1) {
    issues.push('multiple_parent_tasks_signal');
  }
  if (hasNestedBranch(input)) {
    issues.push('nested_branch_signal');
  }
  if (/\bthen\b/i.test(input) && /[,;:]/.test(input)) {
    issues.push('branch_sequence_signal');
  }
  if (/\blike\b.+\bor\b/i.test(input)) {
    issues.push('ambiguous_grouping_signal');
  }
  return unique(issues);
};

const singularize = (input: string): string => {
  if (/ies$/i.test(input) && input.length > 3) {
    return `${input.slice(0, -3)}y`;
  }
  if (/ses$/i.test(input) && input.length > 3) {
    return input.slice(0, -2);
  }
  if (/s$/i.test(input) && !/ss$/i.test(input) && input.length > 2) {
    return input.slice(0, -1);
  }
  return input;
};

const normalizePairedAction = (parentTitle: string): string => {
  const withoutCount = cleanPhrase(
    parentTitle.replace(/\b\d+\b/g, '').replace(/\b(?:one|two|three|four|five)\b(?!-)/gi, ''),
  );
  const parts = withoutCount.split(/\s+/).filter(Boolean);
  if (parts.length === 0) {
    return '';
  }
  parts[parts.length - 1] = singularize(parts[parts.length - 1]);
  return parts.join(' ');
};

const parseChildClause = (clause: string, opts?: TaskDecompositionParseOptions): ChildSeed | null => {
  const parsed = parseTaskInput(clause, opts);
  const target = cleanPhrase(parsed.fields.title === 'Task' ? clause : parsed.fields.title);
  if (!target) {
    return null;
  }
  return {
    title: target,
    target,
    due_date: parsed.fields.due_date,
    due_time: parsed.fields.due_time,
  };
};

const parsePairedTargets = (
  rawInput: string,
  opts?: TaskDecompositionParseOptions,
): TaskDecompositionResult | null => {
  const match = rawInput.match(/^\s*(?<parent>.+?),\s*(?<rest>one\s+for\b.+)$/i);
  if (!match?.groups) {
    return null;
  }

  const parentTitle = cleanPhrase(match.groups.parent);
  const rest = cleanPhrase(match.groups.rest);
  const projectExtraction = extractSharedProject(rest);
  const body = cleanPhrase(projectExtraction.start == null ? rest : rest.slice(0, projectExtraction.start));
  const suffix = projectExtraction.start == null ? '' : rest.slice(projectExtraction.start);
  const suffixTask = suffix ? parseTaskInput(suffix, opts) : null;
  const shared: SharedFields = {
    due_date: suffixTask?.fields.due_date || null,
    due_time: suffixTask?.fields.due_time || null,
    project: projectExtraction.project || suffixTask?.fields.project || null,
  };

  const clauseText = body.replace(/^\s*one\s+for\s+/i, '');
  const clauses = clauseText
    .split(/\s*(?:,|\band\b)\s+one\s+for\s+/i)
    .map(cleanPhrase)
    .filter(Boolean);
  if (clauses.length < 2) {
    return null;
  }

  const action = normalizePairedAction(parentTitle);
  const children = clauses
    .map((clause) => parseChildClause(clause, opts))
    .filter((child): child is ChildSeed => Boolean(child))
    .map((child) => ({
      ...child,
      title: action ? `${action} for ${child.target}` : child.title,
    }));

  if (!shared.due_date && !shared.due_time) {
    const scheduledIndexes = children
      .map((child, index) => ({ child, index }))
      .filter(({ child }) => Boolean(child.due_date || child.due_time));
    const onlyScheduled = scheduledIndexes[0];
    if (scheduledIndexes.length === 1 && onlyScheduled.index === children.length - 1) {
      shared.due_date = onlyScheduled.child.due_date || null;
      shared.due_time = onlyScheduled.child.due_time || null;
      onlyScheduled.child.due_date = null;
      onlyScheduled.child.due_time = null;
    }
  }

  return buildResult('paired_targets', 0.9, parentTitle, shared, children);
};

const parseAlternatePairedTargets = (
  rawInput: string,
  opts?: TaskDecompositionParseOptions,
): TaskDecompositionResult | null => {
  const match = rawInput.match(
    /^\s*(?:for\s+(?:the\s+)?(?<project>.+?project),\s*)?(?<parent>.+?)\s+for\s+(?<targets>[^,]+?),\s*one\s+(?<firstSchedule>.+?)\s+and\s+the\s+other\s+(?<secondSchedule>.+)$/i,
  );
  if (!match?.groups) {
    return null;
  }

  const targets = match.groups.targets.split(/\s+and\s+/i).map(cleanPhrase).filter(Boolean);
  if (targets.length !== 2) {
    return null;
  }

  const parentTitle = cleanPhrase(match.groups.parent);
  const action = normalizePairedAction(parentTitle);
  const firstSchedule = parseTaskInput(match.groups.firstSchedule, opts);
  const secondSchedule = parseTaskInput(match.groups.secondSchedule, opts);
  const shared: SharedFields = {
    due_date: null,
    due_time: null,
    project: match.groups.project ? normalizeProjectValue(`For the ${match.groups.project}`) : null,
  };
  const firstDueTime = firstSchedule.fields.due_time || secondSchedule.fields.due_time;
  const secondDueTime = firstSchedule.fields.due_time ? secondSchedule.fields.due_time : null;
  const children = [
    {
      title: action ? `${action} for ${targets[0]}` : targets[0],
      target: targets[0],
      due_date: firstSchedule.fields.due_date,
      due_time: firstDueTime,
    },
    {
      title: action ? `${action} for ${targets[1]}` : targets[1],
      target: targets[1],
      due_date: secondSchedule.fields.due_date,
      due_time: secondDueTime,
    },
  ];

  return buildResult('paired_targets', 0.86, parentTitle, shared, children);
};

const parseActionList = (input: string): string[] => {
  const cleaned = cleanPhrase(input);
  const goThroughMatch = cleaned.match(/^go\s+through\s+(.+)$/i);
  if (goThroughMatch) {
    return splitList(goThroughMatch[1]).map((action) => `go through ${action}`);
  }

  const verbMatch = cleaned.match(/^(learn|practice|study|review|memorize|transcribe|map|draft|write)\s+(the\s+)?(.+)$/i);
  if (verbMatch) {
    const prefix = `${verbMatch[1].toLowerCase()} ${verbMatch[2] || ''}`;
    return splitList(verbMatch[3]).map((action) => `${prefix}${action}`);
  }

  return splitList(cleaned);
};

const parseStandardCrossProduct = (
  rawInput: string,
): TaskDecompositionResult | null => {
  const text = cleanPhrase(rawInput);
  const shared: SharedFields = { due_date: null, due_time: null, project: null };
  const match = text.match(/^\s*(?<parent>[^:;]+):\s*(?<subjects>.+?)\s*;\s*for\s+each\s+(?:song|tune|standard)\s+(?<actions>.+)$/i);
  if (!match?.groups) {
    return null;
  }

  const subjects = splitList(match.groups.subjects);
  const actions = parseActionList(match.groups.actions);
  if (subjects.length < 2 || actions.length < 2) {
    return null;
  }

  const children = subjects.flatMap((subject) =>
    actions.map((action) => ({
      title: `${action} for ${subject}`,
      subject,
      action,
    })),
  );

  return buildResult('cross_product', 0.88, match.groups.parent, shared, children);
};

const parseReferentialCrossProduct = (
  rawInput: string,
): TaskDecompositionResult | null => {
  const text = cleanPhrase(rawInput);
  const shared: SharedFields = { due_date: null, due_time: null, project: null };
  const match = text.match(
    /^\s*(?:one\s+part\s+of\s+my\s+plan\s+to\s+)?(?<parent>.+?)\s+is\s+to\s+go\s+through\s+(?<actions>.+?)\s+for\s+each\s+of\s+these\s+(?:tunes|songs|standards)\s*:\s*(?<subjects>.+)$/i,
  );
  if (!match?.groups) {
    return null;
  }

  const subjects = splitList(match.groups.subjects);
  const actions = splitList(match.groups.actions).map((action) => `go through ${action}`);
  if (subjects.length < 2 || actions.length < 2) {
    return null;
  }

  const children = subjects.flatMap((subject) =>
    actions.map((action) => ({
      title: `${action} for ${subject}`,
      subject,
      action,
    })),
  );

  return buildResult('cross_product', 0.84, match.groups.parent, shared, children);
};

const parseParallelList = (
  rawInput: string,
  task: TaskParseResult,
): TaskDecompositionResult | null => {
  const { text, shared } = stripSharedFields(rawInput, task);
  if (/\bone\s+for\b|\bfor\s+each\b|;/.test(text)) {
    return null;
  }

  let parentTitle = '';
  let children: string[] = [];
  const colonIndex = text.indexOf(':');
  if (colonIndex >= 0) {
    parentTitle = text.slice(0, colonIndex);
    children = splitList(text.slice(colonIndex + 1));
  } else {
    const parts = splitCommaList(text);
    if (parts.length < 3) {
      return null;
    }
    parentTitle = parts[0];
    children = parts.slice(1);
  }

  if (children.length < 2) {
    return null;
  }

  return buildResult(
    'parallel',
    0.82,
    parentTitle,
    shared,
    children.map((title) => ({ title })),
  );
};

export const parseTaskDecompositionInput = (
  input: string,
  opts?: TaskDecompositionParseOptions,
): TaskDecompositionResult => {
  const rawInput = normalizeWhitespace(input || '');
  if (!rawInput) {
    return emptyResult(['empty_input']);
  }

  const task = parseTaskInput(rawInput, opts);
  const abstentionParent = recoverAbstentionParentTitle(rawInput, task);
  const abstentionShared = sharedFromTask(rawInput, task);
  const hardBlockers = findHardBlockers(rawInput);
  if (hardBlockers.length > 0) {
    return emptyResult(hardBlockers, abstentionParent, abstentionShared);
  }

  return (
    parsePairedTargets(rawInput, opts) ||
    parseAlternatePairedTargets(rawInput, opts) ||
    parseReferentialCrossProduct(rawInput) ||
    parseStandardCrossProduct(rawInput) ||
    parseParallelList(rawInput, task) ||
    emptyResult(['insufficient_decomposition_signals'], abstentionParent, abstentionShared)
  );
};

const isLoadedMlArtifact = (artifact: unknown): artifact is LoadedMlArtifact<'task_decomposition'> =>
  Boolean(artifact && typeof artifact === 'object' && 'parser' in artifact && 'manifest' in artifact && 'tokenizer' in artifact);

export const createTaskDecompositionParser = async (
  config: TaskDecompositionParserConfig = { mode: 'lightweight' },
): Promise<TaskDecompositionParser> => {
  if (!config.mode || config.mode === 'lightweight') {
    return {
      mode: 'lightweight',
      info: {
        parser: 'task_decomposition',
      },
      async parse(input: string, opts?: TaskDecompositionParseOptions): Promise<TaskDecompositionResult> {
        return parseTaskDecompositionInput(input, opts);
      },
      async dispose(): Promise<void> {},
    };
  }

  const artifact = isLoadedMlArtifact(config.artifact)
    ? config.artifact
    : await loadMlArtifact(config.artifact, 'task_decomposition');
  await config.inference.init?.(artifact);

  return {
    mode: 'ml',
    info: {
      parser: 'task_decomposition',
      suite_version: artifact.suite_version,
      artifact_id: artifact.artifact_id,
    },
    async parse(input: string, opts?: TaskDecompositionParseOptions): Promise<TaskDecompositionResult> {
      return parseTaskDecompositionInputWithMl(input, opts, artifact, config.inference);
    },
    async dispose(): Promise<void> {
      await config.inference.dispose?.();
    },
  };
};

export type {
  TaskDecompositionChildTask,
  TaskDecompositionField,
  TaskDecompositionFieldState,
  TaskDecompositionKind,
  TaskDecompositionParseOptions,
  TaskDecompositionParserConfig,
  TaskDecompositionParser,
  TaskDecompositionParserInfo,
  TaskDecompositionPlanTask,
  TaskDecompositionResult,
} from './types.ts';
