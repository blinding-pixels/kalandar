import { classifyRecurrenceException } from '../calendar/recurrenceExceptions.ts';
import type { StructuredRecurrenceException } from '../calendar/types.ts';
import { normalizeWhitespace, parseReferenceDate } from '../shared/utils.ts';
import {
  stripLeadingTitleFiller,
  stripReminderLeadPrefix,
  stripResidualTemporalTokens,
  stripTrailingDanglingPreposition,
} from '../shared/title-utils.ts';
import {
  DEFAULT_KNOWN_ASSIGNEES,
  HIGH_PRIORITY_PATTERNS,
  LEADING_FILLER_PATTERNS,
  LOW_PRIORITY_PATTERNS,
  MEDIUM_PRIORITY_PATTERNS,
  PHRASE_CORRECTIONS,
  STATUS_PATTERNS,
  TRAILING_FILLER_PATTERNS,
} from './constants.ts';
import type { TaskParseContext, TaskParseOptions, TaskParseResult, TaskPriority, TaskStatus } from './types.ts';

type PatternMatch = {
  start: number;
  end: number;
  text: string;
  confidence: number;
  ruleId: string;
  note: string;
};

type FieldName = keyof TaskParseResult['meta']['confidence'];
type SpanLike = { start: number; end: number; text: string };

interface RegexDefinition<T> {
  regex: RegExp;
  value: (match: RegExpExecArray) => T | null;
  confidence: number;
  ruleId: string;
  note: string;
}

interface RegexExtraction<T> {
  value: T;
  span: SpanLike;
  confidence: number;
  ruleId: string;
  note: string;
  working: string;
}

const DURATION_TOKEN = '\\d+\\s*(?:minutes?|mins?|hours?|hrs?)';
const PERSON_TOKEN = "(?:@[a-z0-9_.+-]+|[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?(?:\\s+[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?)?|[a-z]+)";
const PERSON_LIST = `${PERSON_TOKEN}(?:\\s*(?:,|and|&)\\s*${PERSON_TOKEN})*`;
const TEMPORAL_BOUNDARY =
  '(?=\\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\\b|medium\\b|low\\b|urgent\\b|in\\s+progress\\b|blocked\\b|waiting\\b|done\\b|completed?\\b|not\\s+started\\b|todo\\b|tags?\\b|' +
  DURATION_TOKEN +
  '\\b)|\\s*[,.;]|$)';
const CONTEXTUAL_WITH_LEAD_REGEX = /^\s*(?:meeting|meet(?:ing)?|sync|chat|coffee|lunch|dinner|breakfast|call|review)\s+with\b/i;

const normalizeTokenWhitespace = (input: string): string => input.replace(/\s+/g, ' ').trim();

export const clampConfidence = (value: number): number => {
  if (Number.isNaN(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
};

export const createTaskParseResult = (maskedInput: string): TaskParseResult => ({
  fields: {
    title: '',
    assignee: [],
    project: null,
    tags: [],
    priority: null,
    status: null,
    duration: null,
    due_date: null,
    due_time: null,
    recurrence: null,
    recurrence_start: null,
    recurrence_end: null,
    recurrence_exception: [],
  },
  meta: {
    confidence: {
      title: 0,
      assignee: 0,
      project: 0,
      tags: 0,
      priority: 0,
      status: 0,
      duration: 0,
      due_date: 0,
      due_time: 0,
      recurrence: 0,
      recurrence_start: 0,
      recurrence_end: 0,
      recurrence_exception: 0,
    },
    spans: {
      title: [],
      assignee: [],
      project: [],
      tags: [],
      priority: [],
      status: [],
      duration: [],
      due_date: [],
      due_time: [],
      recurrence: [],
      recurrence_start: [],
      recurrence_end: [],
      recurrence_exception: [],
    },
    debugSteps: [],
    maskedInput,
  },
  warnings: null,
  schedule: null,
});

export const createTaskParseContext = (input: string, opts?: TaskParseOptions): TaskParseContext => {
  const rawInput = normalizeWhitespace(input || '');

  return {
    rawInput,
    working: rawInput,
    maskedInput: rawInput,
    now: parseReferenceDate(opts?.now),
    options: {
      timezone: opts?.timezone || 'UTC',
      locale: opts?.locale,
      knownAssignees: opts?.knownAssignees,
      debug: Boolean(opts?.debug),
    },
    result: createTaskParseResult(rawInput),
    state: {
      recurring: false,
    },
  };
};

export const setFieldConfidence = (ctx: TaskParseContext, field: FieldName, value: number): void => {
  const safe = clampConfidence(value);
  if (safe > ctx.result.meta.confidence[field]) {
    ctx.result.meta.confidence[field] = safe;
  }
};

export const addFieldSpan = (ctx: TaskParseContext, field: FieldName, span: SpanLike): void => {
  ctx.result.meta.spans[field].push({
    start: span.start,
    end: span.end,
    text: span.text,
  });
};

export const addDebugStep = (
  ctx: TaskParseContext,
  step: {
    pass: string;
    ruleId: string;
    start: number;
    end: number;
    text: string;
    confidence: number;
    note: string;
  },
): void => {
  ctx.result.meta.debugSteps.push({
    ...step,
    confidence: clampConfidence(step.confidence),
  });
};

const collectPatternMatches = (
  input: string,
  pattern: RegExp,
  ruleId: string,
  confidence: number,
  note: string,
): PatternMatch[] => {
  const matches: PatternMatch[] = [];
  pattern.lastIndex = 0;
  for (const match of input.matchAll(pattern)) {
    const text = match[0] || '';
    const start = match.index ?? -1;
    if (start < 0 || !text) {
      continue;
    }
    matches.push({
      start,
      end: start + text.length,
      text,
      confidence,
      ruleId,
      note,
    });
  }
  pattern.lastIndex = 0;
  return matches;
};

const removeSegment = (working: string, start: number, end: number): string =>
  normalizeWhitespace(`${working.slice(0, start)} ${working.slice(end)}`);

const findBestRegexExtraction = <T>(working: string, definitions: RegexDefinition<T>[]): RegexExtraction<T> | null => {
  let best: { definition: RegexDefinition<T>; match: RegExpExecArray; index: number; value: T } | null = null;

  for (const definition of definitions) {
    definition.regex.lastIndex = 0;
    const match = definition.regex.exec(working);
    definition.regex.lastIndex = 0;
    if (!match || match.index == null) {
      continue;
    }
    const value = definition.value(match);
    if (value === null) {
      continue;
    }
    if (
      !best ||
      match.index < best.index ||
      (match.index === best.index && match[0].length > best.match[0].length)
    ) {
      best = { definition, match, index: match.index, value };
    }
  }

  if (!best) {
    return null;
  }

  const text = best.match[0];
  const start = best.index;
  const end = start + text.length;

  return {
    value: best.value,
    span: { start, end, text },
    confidence: best.definition.confidence,
    ruleId: best.definition.ruleId,
    note: best.definition.note,
    working: removeSegment(working, start, end),
  };
};

export const detectPriority = (
  input: string,
): {
  priority: TaskPriority;
  working: string;
  matches: PatternMatch[];
} => {
  let working = input;
  let priority: TaskPriority = null;
  const matches: PatternMatch[] = [];

  const applyPatterns = (
    patterns: RegExp[],
    nextPriority: Exclude<TaskPriority, null>,
    confidence: number,
    rulePrefix: string,
  ) => {
    let matched = false;
    for (const pattern of patterns) {
      const found = collectPatternMatches(
        working,
        pattern,
        `${rulePrefix}.${pattern.source}`,
        confidence,
        `${nextPriority} priority signal`,
      );
      if (found.length > 0) {
        matches.push(...found);
        matched = true;
      }
      working = working.replace(pattern, ' ');
      pattern.lastIndex = 0;
    }
    if (matched && priority === null) {
      priority = nextPriority;
    }
  };

  applyPatterns(HIGH_PRIORITY_PATTERNS, 'high', 1, 'priority.high');
  if (priority === null) {
    applyPatterns(MEDIUM_PRIORITY_PATTERNS, 'medium', 0.95, 'priority.medium');
  } else {
    for (const pattern of MEDIUM_PRIORITY_PATTERNS) {
      working = working.replace(pattern, ' ');
      pattern.lastIndex = 0;
    }
  }
  if (priority === null) {
    applyPatterns(LOW_PRIORITY_PATTERNS, 'low', 0.95, 'priority.low');
  } else {
    for (const pattern of LOW_PRIORITY_PATTERNS) {
      working = working.replace(pattern, ' ');
      pattern.lastIndex = 0;
    }
  }

  return { priority, working: normalizeWhitespace(working), matches };
};

export const detectStatus = (
  input: string,
): {
  status: TaskStatus;
  working: string;
  span: SpanLike | null;
  confidence: number;
  ruleId: string | null;
  note: string;
} => {
  const extraction = findBestRegexExtraction(
    input,
    STATUS_PATTERNS.map((entry) => ({
      regex: entry.regex,
      value: () => entry.value,
      confidence: 0.96,
      ruleId: `status.${entry.value.replace(/\s+/g, '_')}`,
      note: `captured ${entry.value} status`,
    })),
  );

  if (!extraction) {
    return {
      status: null,
      working: input,
      span: null,
      confidence: 0,
      ruleId: null,
      note: 'no status found',
    };
  }

  return {
    status: extraction.value,
    working: extraction.working,
    span: extraction.span,
    confidence: extraction.confidence,
    ruleId: extraction.ruleId,
    note: extraction.note,
  };
};

const cleanupTagToken = (value: string): string | null => {
  const trimmed = value.trim().replace(/^[,;:]+|[,;:!?]+$/g, '');
  if (!trimmed) {
    return null;
  }
  if (trimmed.startsWith('#')) {
    const normalized = `#${trimmed.slice(1).replace(/[^a-z0-9_-]/gi, '')}`;
    return normalized.length > 1 ? normalized : null;
  }
  const normalized = trimmed.replace(/[^a-z0-9_-]/gi, '');
  return normalized || null;
};

const parseTagList = (value: string): string[] => {
  const hashtagMatches = value.match(/#[a-z0-9][a-z0-9_-]*/gi);
  const rawParts = hashtagMatches && hashtagMatches.length > 0 ? hashtagMatches : value.split(/\s*(?:,|and)\s*/i);
  const tags: string[] = [];
  const seen = new Set<string>();

  for (const part of rawParts) {
    const cleaned = cleanupTagToken(part);
    if (!cleaned) {
      continue;
    }
    const key = cleaned.toLowerCase();
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    tags.push(cleaned);
  }

  return tags;
};

export const extractTags = (
  input: string,
): {
  tags: string[];
  working: string;
  spans: SpanLike[];
  confidence: number;
} => {
  let working = input;
  const tags: string[] = [];
  const spans: SpanLike[] = [];
  const seen = new Set<string>();

  const appendTags = (values: string[]) => {
    for (const value of values) {
      const key = value.toLowerCase();
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      tags.push(value);
    }
  };

  const clauseExtraction = findBestRegexExtraction(working, [
    {
      regex: /\b(?:with\s+)?tags?\s*:?\s*([^.;]+)$/i,
      value: (match) => match[1] || '',
      confidence: 0.98,
      ruleId: 'tags.trailing_clause',
      note: 'captured trailing tags clause',
    },
  ]);

  if (clauseExtraction) {
    appendTags(parseTagList(clauseExtraction.value));
    spans.push(clauseExtraction.span);
    working = clauseExtraction.working;
  }

  const hashtagRegex = /#[a-z0-9][a-z0-9_-]*/gi;
  const hashtagSpans = collectPatternMatches(working, hashtagRegex, 'tags.hashtag', 0.96, 'captured hashtag tag');
  if (hashtagSpans.length > 0) {
    appendTags(
      hashtagSpans
        .map((span) => cleanupTagToken(span.text))
        .filter((value): value is string => Boolean(value)),
    );
    spans.push(...hashtagSpans);
    working = normalizeWhitespace(working.replace(hashtagRegex, ' ').replace(/\b(?:with\s+)?tags?\b[:]?/gi, ' '));
  }

  return {
    tags,
    working,
    spans,
    confidence: tags.length > 0 ? 0.96 : 0,
  };
};

const titleCaseName = (value: string): string =>
  value
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => {
      if (/^@/.test(part) || /@/.test(part)) {
        return part;
      }
      return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase();
    })
    .join(' ');

const cleanNamedValue = (value: string): string => normalizeTokenWhitespace(value.replace(/^[,;:]+|[,;:!?]+$/g, ''));

const splitEntityList = (value: string): string[] =>
  value
    .split(/\s*(?:,|and|&)\s*/i)
    .map((part) => cleanNamedValue(part))
    .filter(Boolean);

const isLikelyPerson = (value: string, knownAssignees: Set<string>): boolean => {
  const cleaned = cleanNamedValue(value);
  if (!cleaned) {
    return false;
  }
  if (/^@[a-z0-9_.+-]+$/i.test(cleaned)) {
    return true;
  }
  if (/^[a-z0-9.+-]+@[a-z0-9.-]+\.[a-z]{2,}$/i.test(cleaned)) {
    return true;
  }
  const normalized = cleaned.toLowerCase();
  if (knownAssignees.has(normalized)) {
    return true;
  }
  if (/^(?:project|tag|tags|today|tomorrow|tonight|next|this|every|daily|weekly|monthly)$/i.test(cleaned)) {
    return false;
  }
  if (/\d/.test(cleaned)) {
    return false;
  }
  const tokens = cleaned.split(/\s+/);
  if (tokens.length > 2) {
    return false;
  }
  return tokens.every((token) => /^[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?$/.test(token));
};

const normalizeAssigneeValue = (value: string, knownAssignees: Set<string>): string | null => {
  const cleaned = cleanNamedValue(value);
  if (!isLikelyPerson(cleaned, knownAssignees)) {
    return null;
  }
  if (/^@/.test(cleaned) || /@/.test(cleaned)) {
    return cleaned;
  }
  return titleCaseName(cleaned);
};

const isLikelyProject = (value: string): boolean => {
  const cleaned = cleanNamedValue(value);
  if (!cleaned) {
    return false;
  }
  if (/^(?:with|to|by|before|every|starting|until|till|high|medium|low|urgent)$/i.test(cleaned)) {
    return false;
  }
  const tokens = cleaned.split(/\s+/);
  if (tokens.length >= 2) {
    return true;
  }
  return /^(?:[A-Z][a-z]+[A-Z][A-Za-z]*|[A-Z]{2,}|[A-Z][a-z]+\d+)$/.test(cleaned);
};

export const extractProject = (
  input: string,
): {
  project: string | null;
  working: string;
  span: SpanLike | null;
  confidence: number;
  ruleId: string | null;
  note: string;
} => {
  const extraction = findBestRegexExtraction(input, [
    {
      regex: new RegExp(`\\b(?:in|under)\\s+project\\s+(.+?)${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) => {
        const candidate = cleanNamedValue(match[1] || '');
        return candidate ? `Project ${candidate}` : null;
      },
      confidence: 0.97,
      ruleId: 'project.explicit_keyword',
      note: 'captured project from explicit project phrase',
    },
    {
      regex: new RegExp(`\\bfor\\s+project\\s+(.+?)${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) => {
        const candidate = cleanNamedValue(match[1] || '');
        return candidate ? `Project ${candidate}` : null;
      },
      confidence: 0.97,
      ruleId: 'project.for_project',
      note: 'captured project from for project phrase',
    },
    {
      regex: new RegExp(`\\bfor\\s+([A-Z][A-Za-z0-9-]*(?:\\s+[A-Z][A-Za-z0-9-]*){1,3})${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) => {
        const candidate = cleanNamedValue(match[1] || '');
        return isLikelyProject(candidate) ? candidate : null;
      },
      confidence: 0.84,
      ruleId: 'project.for_titlecase_phrase',
      note: 'captured probable project from titlecase for-phrase',
    },
  ]);

  if (!extraction) {
    return {
      project: null,
      working: input,
      span: null,
      confidence: 0,
      ruleId: null,
      note: 'no project found',
    };
  }

  return {
    project: extraction.value,
    working: extraction.working,
    span: extraction.span,
    confidence: extraction.confidence,
    ruleId: extraction.ruleId,
    note: extraction.note,
  };
};

export const extractAssignees = (
  input: string,
  knownAssigneesInput?: string[],
): {
  working: string;
  assignees: string[];
  spans: SpanLike[];
} => {
  let working = input;
  const knownAssignees = getKnownAssigneeSet(knownAssigneesInput);
  const assignees: string[] = [];
  const spans: SpanLike[] = [];
  const seen = new Set<string>();

  const addAssignee = (value: string | null) => {
    if (!value) {
      return;
    }
    const key = value.toLowerCase();
    if (seen.has(key)) {
      return;
    }
    seen.add(key);
    assignees.push(value);
  };

  const mentionRegex = /(?<![a-z0-9_.+-])@[a-z0-9_.+-]+/gi;
  const mentionMatches = collectPatternMatches(working, mentionRegex, 'assignee.mention', 1, 'captured explicit @mention assignee');
  if (mentionMatches.length > 0) {
    for (const match of mentionMatches) {
      addAssignee(match.text);
      spans.push(match);
    }
    working = normalizeWhitespace(working.replace(mentionRegex, ' '));
  }

  const clauseDefinitions: RegexDefinition<string[]>[] = [
    {
      regex: new RegExp(`\\bto\\s+(${PERSON_LIST})${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) =>
        splitEntityList(match[1] || '')
          .map((part) => normalizeAssigneeValue(part, knownAssignees))
          .filter((part): part is string => Boolean(part)),
      confidence: 0.9,
      ruleId: 'assignee.to_clause',
      note: 'captured assignee from to-clause',
    },
    {
      regex: new RegExp(`\\bfor\\s+(${PERSON_LIST})${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) => {
        const values = splitEntityList(match[1] || '')
          .map((part) => normalizeAssigneeValue(part, knownAssignees))
          .filter((part): part is string => Boolean(part));
        return values.length > 0 ? values : null;
      },
      confidence: 0.82,
      ruleId: 'assignee.for_clause',
      note: 'captured assignee from for-clause',
    },
    {
      regex: new RegExp(`\\bwith\\s+(${PERSON_LIST})${TEMPORAL_BOUNDARY}`, 'i'),
      value: (match) => {
        if (CONTEXTUAL_WITH_LEAD_REGEX.test(input)) {
          return null;
        }
        const values = splitEntityList(match[1] || '')
          .map((part) => normalizeAssigneeValue(part, knownAssignees))
          .filter((part): part is string => Boolean(part));
        return values.length > 0 ? values : null;
      },
      confidence: 0.82,
      ruleId: 'assignee.with_clause',
      note: 'captured assignee from with-clause',
    },
  ];

  for (;;) {
    const extraction = findBestRegexExtraction(working, clauseDefinitions);
    if (!extraction) {
      break;
    }
    if (extraction.value.length === 0) {
      break;
    }
    for (const value of extraction.value) {
      addAssignee(value);
    }
    spans.push(extraction.span);
    working = extraction.working;
  }

  return {
    working,
    assignees,
    spans,
  };
};

export const mergeStructuredRecurrenceExceptions = (
  current: StructuredRecurrenceException[],
  raw: string,
  now: Date,
  timezone: string,
): StructuredRecurrenceException[] => {
  if (!raw) {
    return current;
  }
  if (current.some((entry) => entry.raw === raw)) {
    return current;
  }

  return [
    ...current,
    classifyRecurrenceException(raw, {
      now,
      timezone,
    }),
  ];
};

const cleanTitlePunctuation = (input: string): string =>
  input
    .replace(/\p{Extended_Pictographic}+/gu, ' ')
    .replace(/[!?.,]{2,}/g, ' ')
    .replace(/[“”"]/g, '')
    .replace(/\s*[,;:]\s*/g, ' ')
    .replace(/\s*-\s*/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const normalizeTitleText = (input: string): string => {
  let output = cleanTitlePunctuation(input);

  for (const pattern of LEADING_FILLER_PATTERNS) {
    output = output.replace(pattern, '');
  }
  for (const pattern of TRAILING_FILLER_PATTERNS) {
    output = output.replace(pattern, '');
  }

  output = output.replace(/\b(?:plz|pls)\b/gi, ' ');
  output = output.replace(/\bcan u\b/gi, ' ');
  output = output.replace(/\bpriority\b/gi, ' ');
  output = stripReminderLeadPrefix(output);

  for (const [pattern, replacement] of PHRASE_CORRECTIONS) {
    output = output.replace(pattern, replacement);
  }

  const temporalStripped = stripResidualTemporalTokens(output);
  const temporalContentRemoved = temporalStripped !== output;
  const preserveTrailingIn = /\b(?:check|log|sign|drop|fill|get)\s+in$/i.test(temporalStripped);
  output = stripLeadingTitleFiller(
    preserveTrailingIn ? temporalStripped : stripTrailingDanglingPreposition(temporalStripped, temporalContentRemoved),
  );

  return normalizeWhitespace(
    output
      .replace(/\b(?:both|but)\b$/i, ' ')
      .replace(/\b(?:for|to|with|by|at|on|from)\b$/i, ' ')
      .replace(/^[,.;:-]+\s*/g, ' ')
      .replace(/\s+[,.;:-]\s*$/g, ' '),
  );
};

export const buildTaskTitle = (input: string): { title: string; confidence: number } => {
  const cleaned = normalizeTitleText(input);
  if (!cleaned) {
    return { title: 'Task', confidence: 0.3 };
  }

  const tokenCount = cleaned.split(/\s+/).filter(Boolean).length;
  const confidence = tokenCount >= 5 ? 0.98 : tokenCount >= 3 ? 0.9 : tokenCount >= 2 ? 0.75 : 0.55;
  return { title: cleaned, confidence };
};

export const getKnownAssigneeSet = (values?: string[]): Set<string> =>
  new Set((values || DEFAULT_KNOWN_ASSIGNEES).map((value) => value.trim().toLowerCase()).filter(Boolean));
