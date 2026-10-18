import { normalizeWhitespace, parseReferenceDate } from '../shared/utils.ts';
import { TITLE_SMALL_WORDS } from '../shared/constants.ts';
import {
  stripLeadingTitleFiller,
  stripReminderLeadPrefix,
  stripResidualTemporalTokens,
  stripTrailingDanglingPreposition,
} from '../shared/title-utils.ts';
import {
  ACRONYM_MAP,
  DEFAULT_TIMEZONE,
  PREFIX_FILLER_PATTERNS,
  PREFIX_PATTERNS,
  PHRASE_CORRECTIONS,
  START_STRIP_PATTERNS,
  TITLE_CORRECTIONS,
  TRAILING_FILLER_PATTERNS,
} from './constants.ts';
import type { ReminderParseContext, ReminderParseOptions, ReminderParseResult } from './types.ts';

export const clampConfidence = (value: number): number => Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));

export const createReminderParseResult = (maskedInput: string): ReminderParseResult => ({
  fields: {
    title: '',
    remind_at: null,
    recurrence: null,
    recurrence_start: null,
    recurrence_end: null,
    recurrence_exception: [],
    recurrence_window: null,
    location: null,
    context_hint: null,
  },
  meta: {
    confidence: {
      title: 0,
      remind_at: 0,
      recurrence: 0,
      recurrence_start: 0,
      recurrence_end: 0,
      recurrence_exception: 0,
      recurrence_window: 0,
      location: 0,
      context_hint: 0,
    },
    spans: {
      title: [],
      remind_at: [],
      recurrence: [],
      recurrence_start: [],
      recurrence_end: [],
      recurrence_exception: [],
      recurrence_window: [],
      location: [],
      context_hint: [],
    },
    debugSteps: [],
    maskedInput,
  },
  warnings: null,
  schedule: null,
});

export const createReminderParseContext = (input: string, opts?: ReminderParseOptions): ReminderParseContext => {
  const rawInput = normalizeWhitespace(input || '');
  const now = parseReferenceDate(opts?.now);
  const timezone = opts?.timezone || DEFAULT_TIMEZONE;
  return {
    rawInput,
    working: rawInput,
    maskedInput: rawInput,
    now,
    options: {
      timezone,
      debug: Boolean(opts?.debug),
    },
    result: createReminderParseResult(rawInput),
  };
};

export const setFieldConfidence = (
  ctx: ReminderParseContext,
  field: keyof ReminderParseResult['meta']['confidence'],
  value: number,
): void => {
  const safe = clampConfidence(value);
  if (safe > ctx.result.meta.confidence[field]) {
    ctx.result.meta.confidence[field] = safe;
  }
};

export const addFieldSpan = (
  ctx: ReminderParseContext,
  field: keyof ReminderParseResult['meta']['spans'],
  span: { start: number; end: number; text: string },
): void => {
  ctx.result.meta.spans[field].push({
    start: span.start,
    end: span.end,
    text: span.text,
  });
};

export const addDebugStep = (
  ctx: ReminderParseContext,
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

export const stripPrefixes = (input: string): { working: string; spans: Array<{ start: number; end: number; text: string }> } => {
  let working = input;
  const spans: Array<{ start: number; end: number; text: string }> = [];
  for (const pattern of PREFIX_FILLER_PATTERNS) {
    const match = working.match(pattern);
    if (match && match[0]) {
      spans.push({ start: match.index ?? 0, end: (match.index ?? 0) + match[0].length, text: match[0] });
    }
    working = working.replace(pattern, '');
  }
  for (const pattern of PREFIX_PATTERNS) {
    const match = working.match(pattern);
    if (match && match[0]) {
      spans.push({ start: match.index ?? 0, end: (match.index ?? 0) + match[0].length, text: match[0] });
    }
    working = working.replace(pattern, ' ');
  }
  for (const pattern of TRAILING_FILLER_PATTERNS) {
    working = working.replace(pattern, '');
  }
  return { working: normalizeWhitespace(working), spans };
};

const cleanTitleWord = (word: string): string => {
  if (/^[A-Z]{2,4}$/.test(word)) {
    return word;
  }
  const lower = word.toLowerCase();
  const corrected = TITLE_CORRECTIONS[lower] ?? lower;
  return corrected;
};

const smartTitleCase = (input: string): string => {
  const words = input
    .split(/\s+/)
    .filter(Boolean);

  let output = words
    .map((word, index) => {
      const cleaned = cleanTitleWord(word);
      if (/^[A-Z]{2,4}$/.test(cleaned)) {
        return cleaned;
      }
      if (ACRONYM_MAP[cleaned.toLowerCase()]) {
        return ACRONYM_MAP[cleaned.toLowerCase()];
      }
      if (/^[a-z]+['’]s$/i.test(cleaned)) {
        const [base] = cleaned.split(/['’]/);
        return `${base.charAt(0).toUpperCase()}${base.slice(1)}'s`;
      }
      if (index > 0 && index < words.length - 1 && TITLE_SMALL_WORDS.has(cleaned.toLowerCase())) {
        return cleaned.toLowerCase();
      }
      return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
    })
    .join(' ');

  for (const [pattern, replacement] of PHRASE_CORRECTIONS) {
    output = output.replace(pattern, replacement);
  }

  return output;
};

export const extractTitle = (input: string): string => {
  let working = input;
  for (const pattern of START_STRIP_PATTERNS) {
    working = working.replace(pattern, '');
  }
  for (const pattern of TRAILING_FILLER_PATTERNS) {
    working = working.replace(pattern, '');
  }

  working = working
    .replace(/(?:\p{Emoji}|\p{Emoji_Modifier}|\p{Emoji_Component}|\uFE0F|\u200D)+/gu, ' ')
    .replace(/[!?.,]{2,}/g, ' ')
    .replace(/(?:^|\s)(?:high|medium|low)\s+priority\b/gi, ' ')
    .replace(/(?:^|\s)(?:urgent|critical|important)\b/gi, ' ')
    .replace(/(?:^|\s)(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b/gi, ' ')
    .replace(/(?:^|\s)@[a-z0-9_.+-]+\b/gi, ' ')
    .replace(/\b\d{1,2}(?::\d{2})?\s*(?:am|pm|a|p)\b/gi, ' ')
    .replace(/\b(?:at\s+)?(?:noon|midnight)\b/gi, ' ')
    .replace(/\b(?:am|pm|a|p)\b/gi, ' ')
    .replace(/\b(?:to|about|that)\b(?=\s*$)/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  working = stripReminderLeadPrefix(working);
  working = stripLeadingTitleFiller(stripTrailingDanglingPreposition(stripResidualTemporalTokens(working)));

  const uniqueWords = working.split(/\s+/).filter(Boolean);
  const deduped: string[] = [];
  for (const word of uniqueWords) {
    if (deduped.length > 0 && deduped[deduped.length - 1].toLowerCase() === word.toLowerCase()) {
      continue;
    }
    deduped.push(word);
  }

  const normalized = deduped
    .join(' ')
    .replace(/\bpickup\b/gi, 'pick up')
    .replace(/\bmums\b/gi, "mum's")
    .replace(/\bbday\b/gi, 'birthday')
    .replace(/\bcehck\b/gi, 'check')
    .replace(/^(?:to(?:\s+the)?|that(?:\s+i)?|about)\b[\s,:-]*/i, '')
    .replace(/\s+/g, ' ')
    .trim();

  return smartTitleCase(normalized || 'Reminder');
};
