import * as chrono from 'chrono-node';

import {
  addDays,
  addMinutes,
  formatDateInTimezone,
  formatDateTimeInTimezone,
  getZonedParts,
  normalizeWhitespace,
  parseReferenceDate,
  toIsoDate,
} from '../shared/utils.ts';
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
  RECURRENCE_DAY_LIST,
  START_STRIP_PATTERNS,
  TIME_PREPROCESS_REPLACEMENTS,
  TITLE_CORRECTIONS,
  TRAILING_FILLER_PATTERNS,
  WEEKDAY_MAP,
  WEEKDAY_SET,
} from './constants.ts';
import type {
  ReminderParseContext,
  ReminderParseOptions,
  ReminderParseResult,
  ReminderRecurrence,
  ReminderRecurrenceFrequency,
} from './types.ts';

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
    state: {
      anchorDayOfMonth: null,
      anchorMonthDay: null,
      recurrenceStartDate: null,
      explicitHour: null,
      explicitMinute: null,
      ambiguousTime: false,
    },
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

const daysInMonth = (year: number, month: number): number => new Date(year, month, 0).getDate();

const withTime = (date: Date, timezone: string, hour: number, minute: number): string => {
  const parts = getZonedParts(date, timezone);
  const isoDate = toIsoDate(parts.year, parts.month, parts.day);
  return `${isoDate}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;
};

const nextOccurrenceAtTime = (now: Date, timezone: string, hour: number, minute: number): string => {
  const nowIso = formatDateTimeInTimezone(now, timezone);
  const todayCandidate = withTime(now, timezone, hour, minute);
  if (todayCandidate >= nowIso) {
    return todayCandidate;
  }
  return withTime(addDays(now, 1), timezone, hour, minute);
};

const shiftedMonthOccurrence = (now: Date, timezone: string, monthDelta: number, hour: number, minute: number): string => {
  const nowParts = getZonedParts(now, timezone);
  const monthIndex = nowParts.month - 1 + monthDelta;
  const yearOffset = Math.floor(monthIndex / 12);
  const safeMonthIndex = ((monthIndex % 12) + 12) % 12;
  const targetYear = nowParts.year + yearOffset;
  const targetMonth = safeMonthIndex + 1;
  const targetDay = Math.min(nowParts.day, daysInMonth(targetYear, targetMonth));
  return `${toIsoDate(targetYear, targetMonth, targetDay)}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;
};

const toWeekday = (token: string): string | null => {
  const normalized = token.toLowerCase();
  if (/^mon/.test(normalized)) return 'monday';
  if (/^tue/.test(normalized)) return 'tuesday';
  if (/^wed/.test(normalized)) return 'wednesday';
  if (/^thu/.test(normalized)) return 'thursday';
  if (/^fri/.test(normalized)) return 'friday';
  if (/^sat/.test(normalized)) return 'saturday';
  if (/^sun/.test(normalized)) return 'sunday';
  return null;
};

const parseMonthToken = (token: string): number | null => {
  const normalized = token.toLowerCase();
  const months: Record<string, number> = {
    january: 1,
    jan: 1,
    february: 2,
    feb: 2,
    march: 3,
    mar: 3,
    april: 4,
    apr: 4,
    may: 5,
    june: 6,
    jun: 6,
    july: 7,
    jul: 7,
    august: 8,
    aug: 8,
    september: 9,
    sept: 9,
    sep: 9,
    october: 10,
    oct: 10,
    november: 11,
    nov: 11,
    december: 12,
    dec: 12,
  };
  return months[normalized] ?? null;
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

export const extractRecurrence = (input: string): {
  recurrence: ReminderRecurrence | null;
  working: string;
  anchorDayOfMonth: number | null;
  anchorMonthDay: { month: number; day: number } | null;
  span: { start: number; end: number; text: string } | null;
} => {
  let working = input;
  let recurrence: ReminderRecurrence | null = null;
  let anchorDayOfMonth: number | null = null;
  let anchorMonthDay: { month: number; day: number } | null = null;
  let span: { start: number; end: number; text: string } | null = null;

  const makeRecurrence = (
    frequency: ReminderRecurrenceFrequency,
    interval: number,
    days: string[] | null,
  ): ReminderRecurrence => ({ frequency, interval, days });

  const multiDayMatch = working.match(
    new RegExp(`\\bevery\\s+(${RECURRENCE_DAY_LIST})(?:\\s*(?:,|and)\\s*(${RECURRENCE_DAY_LIST}))+(?=\\b|\\s)`, 'i'),
  );
  if (multiDayMatch) {
    const matches = Array.from(multiDayMatch[0].matchAll(new RegExp(RECURRENCE_DAY_LIST, 'gi')))
      .map((entry) => toWeekday(entry[0]))
      .filter((value): value is string => Boolean(value));
    recurrence = makeRecurrence('weekly', 1, [...new Set(matches)]);
    span = {
      start: multiDayMatch.index ?? 0,
      end: (multiDayMatch.index ?? 0) + multiDayMatch[0].length,
      text: multiDayMatch[0],
    };
    working = normalizeWhitespace(working.replace(multiDayMatch[0], ' '));
  }

  const weekdayPatterns: Array<{ regex: RegExp; frequency: ReminderRecurrenceFrequency; interval: number | null }> = [
    { regex: /\bevery\s+weekday\b/i, frequency: 'weekly', interval: 1 },
    { regex: /\bevery\s+other\s+day\b/i, frequency: 'daily', interval: 2 },
    { regex: /\bevery\s+other\s+week\b/i, frequency: 'weekly', interval: 2 },
    { regex: /\bevery\s+other\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/i, frequency: 'weekly', interval: 2 },
    { regex: /\bevery\s+(\d+)\s+days\b/i, frequency: 'daily', interval: null },
    { regex: /\bevery\s+(\d+)\s+weeks\b/i, frequency: 'weekly', interval: null },
    { regex: /\bevery\s+(\d+)\s+months\b/i, frequency: 'monthly', interval: null },
    { regex: /\bevery\s+week\s+on\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/i, frequency: 'weekly', interval: 1 },
    { regex: /\bevery\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/i, frequency: 'weekly', interval: 1 },
    { regex: /\bevery\s+day\b/i, frequency: 'daily', interval: 1 },
    { regex: /\bevery\s+month\b/i, frequency: 'monthly', interval: 1 },
    { regex: /^\s*daily\b/i, frequency: 'daily', interval: 1 },
    { regex: /^\s*weekly\b/i, frequency: 'weekly', interval: 1 },
    { regex: /^\s*monthly\b/i, frequency: 'monthly', interval: 1 },
    { regex: /^\s*yearly\b/i, frequency: 'yearly', interval: 1 },
    { regex: /\bevry\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/i, frequency: 'weekly', interval: 1 },
  ];

  for (const rule of weekdayPatterns) {
    const match = working.match(rule.regex);
    if (!match) {
      continue;
    }

    if (!recurrence) {
      if (rule.regex.source.includes('(\\d+)')) {
        const parsedAmount = Number(match[1]);
        const interval = Number.isInteger(parsedAmount) && parsedAmount > 0
          ? parsedAmount
          : (rule.interval ?? 1);
        recurrence = makeRecurrence(
          rule.frequency === 'monthly' ? 'monthly' : rule.frequency === 'weekly' ? 'weekly' : 'daily',
          interval,
          null,
        );
      } else if (match[1] && WEEKDAY_SET.has(match[1].toLowerCase())) {
        recurrence = makeRecurrence(rule.frequency, rule.interval ?? 1, [toWeekday(match[1]) || 'monday']);
      } else if (rule.frequency === 'weekly' && /weekday/i.test(match[0])) {
        recurrence = makeRecurrence('weekly', 1, ['monday', 'tuesday', 'wednesday', 'thursday', 'friday']);
      } else {
        recurrence = makeRecurrence(rule.frequency, rule.interval ?? 1, null);
      }
    }

    span = {
      start: match.index ?? 0,
      end: (match.index ?? 0) + match[0].length,
      text: match[0],
    };
    working = normalizeWhitespace(working.replace(match[0], ' '));
  }

  const monthlyDayMatch = working.match(/\bon\s+the\s+(\d{1,2})(?:st|nd|rd|th)\b/i);
  if (recurrence && recurrence.frequency === 'monthly' && monthlyDayMatch) {
    anchorDayOfMonth = Number(monthlyDayMatch[1]);
    working = normalizeWhitespace(working.replace(monthlyDayMatch[0], ' '));
  }

  const yearlyMonthDayMatch = working.match(/\bon\s+([A-Za-z]+)\s+(\d{1,2})\b|\b([A-Za-z]+)\s+(\d{1,2})\b/i);
  if (recurrence && recurrence.frequency === 'yearly' && yearlyMonthDayMatch) {
    const monthToken = yearlyMonthDayMatch[1] || yearlyMonthDayMatch[3];
    const dayToken = yearlyMonthDayMatch[2] || yearlyMonthDayMatch[4];
    const month = parseMonthToken(monthToken);
    if (month) {
      anchorMonthDay = { month, day: Number(dayToken) };
      working = normalizeWhitespace(working.replace(yearlyMonthDayMatch[0], ' '));
    }
  }

  if (recurrence && recurrence.frequency === 'weekly' && recurrence.days === null) {
    const standaloneDay = working.match(new RegExp(`\\b(${RECURRENCE_DAY_LIST})\\b`, 'i'));
    if (standaloneDay) {
      const day = toWeekday(standaloneDay[1]);
      if (day) {
        recurrence = { frequency: recurrence.frequency, interval: recurrence.interval, days: [day] };
        working = normalizeWhitespace(working.replace(standaloneDay[0], ' '));
      }
    }
  }

  return {
    recurrence,
    working,
    anchorDayOfMonth,
    anchorMonthDay,
    span,
  };
};

export const preprocessTimeText = (input: string): string => {
  let working = input;
  for (const [pattern, replacement] of TIME_PREPROCESS_REPLACEMENTS) {
    if (typeof replacement === 'string') {
      working = working.replace(pattern, replacement);
    } else {
      working = working.replace(pattern, (...args) => replacement(...args.map((value) => String(value))));
    }
  }
  return normalizeWhitespace(working).replace(/\bb4\b/gi, 'before');
};

const findDateLike = (working: string, now: Date, timezone: string): string | null => {
  const ordinalMatch = working.match(/\bon the (\d{1,2})(?:st|nd|rd|th)\b/i);
  if (ordinalMatch) {
    const day = Number(ordinalMatch[1]);
    const parts = getZonedParts(now, timezone);
    const currentMonthCandidate = `${toIsoDate(parts.year, parts.month, day)}T09:00:00`;
    if (currentMonthCandidate >= formatDateTimeInTimezone(now, timezone)) {
      return currentMonthCandidate;
    }
    const nextMonth = parts.month === 12 ? 1 : parts.month + 1;
    const year = parts.month === 12 ? parts.year + 1 : parts.year;
    return `${toIsoDate(year, nextMonth, day)}T09:00:00`;
  }

  const parsed = chrono.parse(working, now, { forwardDate: true });
  const best = parsed.find((entry) => entry.start.isCertain('day') || entry.start.isCertain('weekday') || entry.start.isCertain('month'));
  if (!best) {
    return null;
  }
  return withTime(best.start.date(), timezone, 9, 0);
};

const upcomingWeekend = (now: Date, timezone: string): string => {
  const parts = getZonedParts(now, timezone);
  if (parts.weekday === 6 && parts.hour < 10) {
    return withTime(now, timezone, 10, 0);
  }
  if (parts.weekday === 6) {
    return withTime(addDays(now, 1), timezone, 10, 0);
  }
  const daysUntilSaturday = (6 - parts.weekday + 7) % 7;
  return withTime(addDays(now, daysUntilSaturday), timezone, 10, 0);
};

const endOfMonthReminderAt = (now: Date, timezone: string): string => {
  const parts = getZonedParts(now, timezone);
  const nowIso = formatDateTimeInTimezone(now, timezone);
  const thisMonthCandidate = `${toIsoDate(parts.year, parts.month, daysInMonth(parts.year, parts.month))}T09:00:00`;
  if (thisMonthCandidate >= nowIso) {
    return thisMonthCandidate;
  }
  const nextMonth = parts.month === 12 ? 1 : parts.month + 1;
  const nextYear = parts.month === 12 ? parts.year + 1 : parts.year;
  return `${toIsoDate(nextYear, nextMonth, daysInMonth(nextYear, nextMonth))}T09:00:00`;
};

const endOfWeekReminderAt = (now: Date, timezone: string): string => {
  const parts = getZonedParts(now, timezone);
  let delta = (5 - parts.weekday + 7) % 7;
  const nowIso = formatDateTimeInTimezone(now, timezone);
  let candidate = withTime(addDays(now, delta), timezone, 17, 0);
  if (candidate < nowIso) {
    delta += 7;
    candidate = withTime(addDays(now, delta), timezone, 17, 0);
  }
  return candidate;
};

export const applyRelativeTimeRules = (
  workingInput: string,
  now: Date,
  timezone: string,
): {
  working: string;
  remindAt: string | null;
  contextHint: string | null;
  explicitHour: number | null;
  explicitMinute: number | null;
  span: { start: number; end: number; text: string } | null;
  confidence: number;
} => {
  let working = preprocessTimeText(workingInput);
  const hasExplicitClockTime =
    /\b\d{1,2}(?::\d{2})?\s*(?:am|pm)\b/i.test(working) ||
    /\bat\s+\d{1,2}(?::\d{2})?\b/i.test(working) ||
    /\b(?:noon|midnight)\b/i.test(working);
  let remindAt: string | null = null;
  let contextHint: string | null = null;
  let explicitHour: number | null = null;
  let explicitMinute: number | null = null;
  let span: { start: number; end: number; text: string } | null = null;
  let confidence = 0;

  if (hasExplicitClockTime) {
    const contextMatch =
      working.match(/\bfirst thing(?: in the morning)?\b/i) ||
      working.match(/\bafter lunch\b/i) ||
      working.match(/\b(?:mid-afternoon|mid afternoon|arvo|this afternoon)\b/i) ||
      working.match(/\b(?:end of day|eod|before I leave today)\b/i) ||
      working.match(/\bthis weekend\b/i) ||
      working.match(/\btonight\b/i) ||
      working.match(/\bsat(?:urday)? morning\b/i) ||
      working.match(/\b(?:tomorrow|monday|tuesday|wednesday|thursday|friday|saturday|sunday)?\s*morning\b/i);

    if (contextMatch) {
      span = {
        start: contextMatch.index ?? 0,
        end: (contextMatch.index ?? 0) + contextMatch[0].length,
        text: contextMatch[0],
      };
      contextHint = normalizeWhitespace(contextMatch[0]).toLowerCase();
      confidence = 0.6;
    }
  }

  const consume = (regex: RegExp): RegExpMatchArray | null => {
    const match = working.match(regex);
    if (!match) {
      return null;
    }
    let removeStart = match.index ?? 0;
    let removeEnd = (match.index ?? 0) + match[0].length;
    const before = working.slice(0, removeStart);
    const leadingTemporalMarker = before.match(/\b(?:by|before|after|until|on|at)\b\s*$/i);
    if (leadingTemporalMarker) {
      removeStart -= leadingTemporalMarker[0].length;
    }
    const trailingPunctuationMatch = working.slice(removeEnd).match(/^\s*[.,;:!?]+/);
    if (trailingPunctuationMatch) {
      removeEnd += trailingPunctuationMatch[0].length;
    }
    span = {
      start: removeStart,
      end: removeEnd,
      text: working.slice(removeStart, removeEnd),
    };
    working = normalizeWhitespace(`${working.slice(0, removeStart)} ${working.slice(removeEnd)}`);
    return match;
  };

  if (!hasExplicitClockTime) {
    const firstThing = consume(/\bfirst thing(?: in the morning)?\b/i);
    if (firstThing) {
      remindAt = withTime(addDays(now, 1), timezone, 9, 0);
      contextHint = 'first thing in the morning';
      explicitHour = 9;
      explicitMinute = 0;
      confidence = 0.86;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const afterLunch = consume(/\bafter lunch\b/i);
    if (afterLunch) {
      remindAt = nextOccurrenceAtTime(now, timezone, 13, 0);
      contextHint = 'after lunch';
      explicitHour = 13;
      explicitMinute = 0;
      confidence = 0.82;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const afternoon = consume(/\b(?:mid-afternoon|mid afternoon|arvo|this afternoon)\b/i);
    if (afternoon) {
      const dateLike = findDateLike(working, now, timezone);
      remindAt = dateLike ? `${dateLike.slice(0, 10)}T15:00:00` : nextOccurrenceAtTime(now, timezone, 15, 0);
      contextHint = afternoon[0];
      explicitHour = 15;
      explicitMinute = 0;
      confidence = 0.78;
    }
  }

  if (!remindAt) {
    const endOfMonth = consume(/\bend of (?:the )?month\b/i);
    if (endOfMonth) {
      remindAt = endOfMonthReminderAt(now, timezone);
      contextHint = 'end of month';
      if (!hasExplicitClockTime) {
        explicitHour = 9;
        explicitMinute = 0;
      }
      confidence = 0.84;
    }
  }

  if (!remindAt) {
    const endOfWeek = consume(/\b(?:end of (?:the )?week|eow)\b/i);
    if (endOfWeek) {
      remindAt = endOfWeekReminderAt(now, timezone);
      contextHint = 'end of week';
      if (!hasExplicitClockTime) {
        explicitHour = 17;
        explicitMinute = 0;
      }
      confidence = 0.84;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const eod = consume(/\b(?:end of day|eod|before I leave today)\b/i);
    if (eod) {
      remindAt = nextOccurrenceAtTime(now, timezone, 17, 0);
      contextHint = eod[0].toLowerCase();
      explicitHour = 17;
      explicitMinute = 0;
      confidence = 0.8;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const weekend = consume(/\bthis weekend\b/i);
    if (weekend) {
      remindAt = upcomingWeekend(now, timezone);
      contextHint = 'this weekend';
      explicitHour = 10;
      explicitMinute = 0;
      confidence = 0.72;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const fortnight = consume(/\bin a fortnight\b/i);
    if (fortnight) {
      remindAt = withTime(addDays(now, 14), timezone, 9, 0);
      contextHint = 'in a fortnight';
      explicitHour = 9;
      explicitMinute = 0;
      confidence = 0.74;
    }
  }

  if (!remindAt) {
    const halfHour = consume(/\bin half (?:an )?hour\b/i);
    if (halfHour) {
      remindAt = formatDateTimeInTimezone(addMinutes(now, 30), timezone);
      contextHint = 'in half hour';
      confidence = 0.9;
      const parts = getZonedParts(new Date(remindAt), timezone);
      explicitHour = parts.hour;
      explicitMinute = parts.minute;
    }
  }

  if (!remindAt) {
    const relative = consume(/\bin (\d+)\s+(minutes|minute|hours|hour)\b/i);
    if (relative) {
      const amount = Number(relative[1]);
      const unit = relative[2].toLowerCase();
      remindAt = formatDateTimeInTimezone(addMinutes(now, unit.startsWith('hour') ? amount * 60 : amount), timezone);
      contextHint = null;
      confidence = 0.92;
      const parts = getZonedParts(new Date(remindAt), timezone);
      explicitHour = parts.hour;
      explicitMinute = parts.minute;
    }
  }

  if (!remindAt) {
    const relativeDate = consume(/\bin (\d+)\s+(days|day|weeks|week|months|month)\b/i);
    if (relativeDate) {
      const amount = Number(relativeDate[1]);
      const safeAmount = Number.isInteger(amount) && amount > 0 ? amount : 1;
      const unit = relativeDate[2].toLowerCase();

      if (unit.startsWith('month')) {
        remindAt = shiftedMonthOccurrence(now, timezone, safeAmount, 9, 0);
      } else if (unit.startsWith('week')) {
        remindAt = withTime(addDays(now, safeAmount * 7), timezone, 9, 0);
      } else {
        remindAt = withTime(addDays(now, safeAmount), timezone, 9, 0);
      }

      contextHint = normalizeWhitespace(relativeDate[0]).toLowerCase();
      if (!hasExplicitClockTime) {
        explicitHour = 9;
        explicitMinute = 0;
      }
      confidence = 0.84;
    }
  }

  if (!remindAt) {
    const nextMonth = consume(/\bnext\s+month\b/i);
    if (nextMonth) {
      remindAt = shiftedMonthOccurrence(now, timezone, 1, 9, 0);
      contextHint = 'next month';
      if (!hasExplicitClockTime) {
        explicitHour = 9;
        explicitMinute = 0;
      }
      confidence = 0.84;
    }
  }

  if (!remindAt) {
    const lunchTomorrow = consume(/\baround 12pm tomorrow\b/i);
    if (lunchTomorrow) {
      remindAt = withTime(addDays(now, 1), timezone, 12, 0);
      contextHint = 'around lunchtime tomorrow';
      explicitHour = 12;
      explicitMinute = 0;
      confidence = 0.8;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const tonight = consume(/\btonight\b/i);
    if (tonight) {
      remindAt = withTime(now, timezone, 20, 0);
      contextHint = 'tonight';
      explicitHour = 20;
      explicitMinute = 0;
      confidence = 0.78;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const satMorning = consume(/\bsat(?:urday)? morning\b/i);
    if (satMorning && getZonedParts(now, timezone).weekday === 6) {
      const parts = getZonedParts(now, timezone);
      remindAt = parts.hour < 8 ? withTime(now, timezone, 8, 0) : withTime(addDays(now, 1), timezone, 8, 0);
      contextHint = 'Saturday morning';
      explicitHour = 8;
      explicitMinute = 0;
      confidence = 0.82;
    }
  }

  if (!remindAt && !hasExplicitClockTime) {
    const morning = consume(/\b(?:tomorrow|monday|tuesday|wednesday|thursday|friday|saturday|sunday)?\s*morning\b/i);
    if (morning) {
      const normalized = normalizeWhitespace(morning[0]).toLowerCase();
      const dateLike = findDateLike(working, now, timezone);
      if (dateLike) {
        const hour = /\bbefore\b/i.test(working) ? 8 : 9;
        remindAt = `${dateLike.slice(0, 10)}T${String(hour).padStart(2, '0')}:00:00`;
      } else {
        const nowParts = getZonedParts(now, timezone);
        remindAt = nowParts.hour < 9 ? withTime(now, timezone, 9, 0) : withTime(addDays(now, 1), timezone, 9, 0);
      }
      contextHint = normalized;
      explicitHour = 9;
      explicitMinute = 0;
      confidence = 0.7;
    }
  }

  if (!remindAt) {
    const unresolved = consume(/\b(before the meeting|b4 the meeting|after the standup|before flight|before leaving|on the way home)\b/i);
    if (unresolved) {
      contextHint = unresolved[0].toLowerCase();
      confidence = 0.5;
    }
  }

  return {
    working: normalizeWhitespace(working),
    remindAt,
    contextHint,
    explicitHour,
    explicitMinute,
    span,
    confidence,
  };
};

const parseClockToken = (token: string): { hour: number; minute: number } | null => {
  const normalized = token.trim().toLowerCase();
  if (normalized === 'noon') {
    return { hour: 12, minute: 0 };
  }
  if (normalized === 'midnight') {
    return { hour: 0, minute: 0 };
  }

  const match = normalized.match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/);
  if (!match) {
    return null;
  }
  let hour = Number(match[1]);
  const minute = Number(match[2] || '0');
  const meridiem = match[3] || null;

  if (meridiem === 'pm' && hour < 12) {
    hour += 12;
  }
  if (meridiem === 'am' && hour === 12) {
    hour = 0;
  }

  if (hour > 23 || minute > 59) {
    return null;
  }
  return { hour, minute };
};

const inferDateForTime = (working: string, now: Date, timezone: string, hour: number, minute: number): string => {
  const dateLike = findDateLike(working, now, timezone);
  if (dateLike) {
    return `${dateLike.slice(0, 10)}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;
  }

  const nowParts = getZonedParts(now, timezone);
  const todayISO = toIsoDate(nowParts.year, nowParts.month, nowParts.day);
  const todayCandidate = `${todayISO}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;
  if (todayCandidate >= formatDateTimeInTimezone(now, timezone)) {
    return todayCandidate;
  }
  const tomorrow = addDays(now, 1);
  const tomorrowParts = getZonedParts(tomorrow, timezone);
  return `${toIsoDate(tomorrowParts.year, tomorrowParts.month, tomorrowParts.day)}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;
};

export const applyAbsoluteTimeRules = (
  workingInput: string,
  now: Date,
  timezone: string,
): {
  working: string;
  remindAt: string | null;
  contextHint: string | null;
  explicitHour: number | null;
  explicitMinute: number | null;
  span: { start: number; end: number; text: string } | null;
  confidence: number;
  ambiguousTime: boolean;
} => {
  let working = preprocessTimeText(workingInput);
  const match = working.match(/\bat\s+(noon|midnight|\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\b|\b(noon|midnight|\d{1,2}(?::\d{2})?\s*(?:am|pm))\b/i);
  if (!match) {
    return {
      working: normalizeWhitespace(working),
      remindAt: null,
      contextHint: null,
      explicitHour: null,
      explicitMinute: null,
      span: null,
      confidence: 0,
      ambiguousTime: false,
    };
  }

  const token = match[1] || match[2] || '';
  const parsed = parseClockToken(token);
  if (!parsed) {
    return {
      working: normalizeWhitespace(working),
      remindAt: null,
      contextHint: null,
      explicitHour: null,
      explicitMinute: null,
      span: null,
      confidence: 0,
      ambiguousTime: false,
    };
  }

  const remindAt = inferDateForTime(working, now, timezone, parsed.hour, parsed.minute);
  const span = {
    start: match.index ?? 0,
    end: (match.index ?? 0) + match[0].length,
    text: match[0],
  };
  working = normalizeWhitespace(working.replace(match[0], ' '));

  return {
    working,
    remindAt,
    contextHint: null,
    explicitHour: parsed.hour,
    explicitMinute: parsed.minute,
    span,
    confidence: /(?:am|pm|noon|midnight)/i.test(token) ? 1 : 0.65,
    ambiguousTime: /\d{1,2}$/.test(token.trim()),
  };
};

const mergeDateIntoReminderAt = (existingReminderAt: string, dateISO: string): string => `${dateISO}${existingReminderAt.slice(10)}`;

const stripDateSpanWithTemporalMarker = (
  working: string,
  start: number,
  end: number,
): { working: string; start: number; end: number; text: string } => {
  let removeStart = start;
  let removeEnd = end;

  const before = working.slice(0, start);
  const leadingTemporalMarker = before.match(/\b(?:by|before|after|until|on|at)\b\s*$/i);
  if (leadingTemporalMarker) {
    removeStart = start - leadingTemporalMarker[0].length;
  }

  const after = working.slice(end);
  const trailingPunctuationMatch = after.match(/^\s*[.,;:!?]+/);
  if (trailingPunctuationMatch) {
    removeEnd = end + trailingPunctuationMatch[0].length;
  }

  return {
    working: normalizeWhitespace(`${working.slice(0, removeStart)} ${working.slice(removeEnd)}`),
    start: removeStart,
    end: removeEnd,
    text: working.slice(removeStart, removeEnd),
  };
};

export const applyNamedDateRules = (
  workingInput: string,
  now: Date,
  timezone: string,
  existingReminderAt: string | null,
): {
  working: string;
  remindAt: string | null;
  contextHint: string | null;
  span: { start: number; end: number; text: string } | null;
  confidence: number;
} => {
  let working = preprocessTimeText(workingInput);

  const ordinalMatch = working.match(/\bon the (\d{1,2})(?:st|nd|rd|th)\b/i);
  if (ordinalMatch) {
    const day = Number(ordinalMatch[1]);
    const parts = getZonedParts(now, timezone);
    const currentMonthCandidate = `${toIsoDate(parts.year, parts.month, day)}T09:00:00`;
    const dateISO =
      currentMonthCandidate >= formatDateTimeInTimezone(now, timezone)
        ? currentMonthCandidate.slice(0, 10)
        : toIsoDate(parts.month === 12 ? parts.year + 1 : parts.year, parts.month === 12 ? 1 : parts.month + 1, day);

    const remindAt = existingReminderAt ? mergeDateIntoReminderAt(existingReminderAt, dateISO) : `${dateISO}T09:00:00`;
    const span = {
      start: ordinalMatch.index ?? 0,
      end: (ordinalMatch.index ?? 0) + ordinalMatch[0].length,
      text: ordinalMatch[0],
    };
    working = normalizeWhitespace(working.replace(ordinalMatch[0], ' '));
    return {
      working,
      remindAt,
      contextHint: ordinalMatch[0],
      span,
      confidence: 0.8,
    };
  }

  const parsed = chrono.parse(working, now, { forwardDate: true });
  const best = parsed.find((entry) => entry.start.isCertain('day') || entry.start.isCertain('weekday') || entry.start.isCertain('month'));
  if (!best) {
    return {
      working: normalizeWhitespace(working),
      remindAt: existingReminderAt,
      contextHint: null,
      span: null,
      confidence: 0,
    };
  }

  const dateISO = toIsoDate(
    best.start.get('year') ?? getZonedParts(best.start.date(), timezone).year,
    best.start.get('month') ?? getZonedParts(best.start.date(), timezone).month,
    best.start.get('day') ?? getZonedParts(best.start.date(), timezone).day,
  );

  const remindAt = existingReminderAt ? mergeDateIntoReminderAt(existingReminderAt, dateISO) : `${dateISO}T09:00:00`;
  const span = {
    start: best.index ?? 0,
    end: (best.index ?? 0) + best.text.length,
    text: best.text,
  };
  const stripped = stripDateSpanWithTemporalMarker(working, span.start, span.end);
  working = stripped.working;

  return {
    working,
    remindAt,
    contextHint: stripped.text,
    span: {
      start: stripped.start,
      end: stripped.end,
      text: stripped.text,
    },
    confidence: best.start.isCertain('day') ? 0.85 : 0.72,
  };
};

export const chronoFallback = (
  workingInput: string,
  now: Date,
  timezone: string,
): {
  working: string;
  remindAt: string | null;
  span: { start: number; end: number; text: string } | null;
  confidence: number;
  explicitHour: number | null;
  explicitMinute: number | null;
} => {
  const working = preprocessTimeText(workingInput);
  const parsed = chrono.parse(working, now, { forwardDate: true });
  const best = parsed[0];
  if (!best) {
    return {
      working: normalizeWhitespace(working),
      remindAt: null,
      span: null,
      confidence: 0,
      explicitHour: null,
      explicitMinute: null,
    };
  }

  const date = best.start.date();
  const hasHour = best.start.isCertain('hour');
  const parts = getZonedParts(date, timezone);
  const hour = hasHour ? parts.hour : 9;
  const minute = hasHour ? parts.minute : 0;
  const remindAt = withTime(date, timezone, hour, minute);
  const stripped = stripDateSpanWithTemporalMarker(working, best.index ?? 0, (best.index ?? 0) + best.text.length);
  const nextWorking = stripped.working;

  return {
    working: nextWorking,
    remindAt,
    span: {
      start: stripped.start,
      end: stripped.end,
      text: stripped.text,
    },
    confidence: hasHour ? 0.86 : 0.68,
    explicitHour: hour,
    explicitMinute: minute,
  };
};

const parseIsoDateParts = (isoDate: string): { year: number; month: number; day: number } => {
  const [year, month, day] = isoDate.split('-').map((value) => Number(value));
  return { year, month, day };
};

const addDaysToIsoDate = (isoDate: string, days: number): string => {
  const { year, month, day } = parseIsoDateParts(isoDate);
  const shifted = new Date(Date.UTC(year, month - 1, day));
  shifted.setUTCDate(shifted.getUTCDate() + days);
  return toIsoDate(shifted.getUTCFullYear(), shifted.getUTCMonth() + 1, shifted.getUTCDate());
};

const weekdayOfIsoDate = (isoDate: string): number => {
  const { year, month, day } = parseIsoDateParts(isoDate);
  return new Date(Date.UTC(year, month - 1, day)).getUTCDay();
};

const withIsoDateTime = (isoDate: string, hour: number, minute: number): string =>
  `${isoDate}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}:00`;

const withNextMonth = (year: number, month: number): { year: number; month: number } =>
  month === 12 ? { year: year + 1, month: 1 } : { year, month: month + 1 };

const monthlyOccurrenceFromTemplate = (
  searchDateIso: string,
  searchTimeMinutes: number,
  templateDayOfMonth: number,
  hour: number,
  minute: number,
): string => {
  const { year, month } = parseIsoDateParts(searchDateIso);
  const currentSafeDay = Math.min(templateDayOfMonth, daysInMonth(year, month));
  const currentCandidate = withIsoDateTime(toIsoDate(year, month, currentSafeDay), hour, minute);
  const searchCandidate = withIsoDateTime(
    searchDateIso,
    Math.floor(searchTimeMinutes / 60),
    searchTimeMinutes % 60,
  );

  if (currentCandidate >= searchCandidate) {
    return currentCandidate;
  }

  const nextMonth = withNextMonth(year, month);
  const nextSafeDay = Math.min(templateDayOfMonth, daysInMonth(nextMonth.year, nextMonth.month));
  return withIsoDateTime(toIsoDate(nextMonth.year, nextMonth.month, nextSafeDay), hour, minute);
};

const yearlyOccurrenceFromTemplate = (
  searchDateIso: string,
  searchTimeMinutes: number,
  templateMonth: number,
  templateDay: number,
  hour: number,
  minute: number,
): string => {
  const { year } = parseIsoDateParts(searchDateIso);
  const currentSafeDay = Math.min(templateDay, daysInMonth(year, templateMonth));
  const currentCandidate = withIsoDateTime(toIsoDate(year, templateMonth, currentSafeDay), hour, minute);
  const searchCandidate = withIsoDateTime(
    searchDateIso,
    Math.floor(searchTimeMinutes / 60),
    searchTimeMinutes % 60,
  );

  if (currentCandidate >= searchCandidate) {
    return currentCandidate;
  }

  const nextSafeDay = Math.min(templateDay, daysInMonth(year + 1, templateMonth));
  return withIsoDateTime(toIsoDate(year + 1, templateMonth, nextSafeDay), hour, minute);
};

export const resolveRecurrenceStartDate = (value: string, now: Date, timezone: string): string | null => {
  const normalized = normalizeWhitespace(value.replace(/^(?:starting|beginning|from)\s+/i, ''));
  if (!normalized) {
    return null;
  }

  if (/^today$/i.test(normalized) || /^this\s+(?:week|month|year)$/i.test(normalized)) {
    return formatDateInTimezone(now, timezone);
  }

  if (/^tomorrow$/i.test(normalized)) {
    return formatDateInTimezone(addDays(now, 1), timezone);
  }

  if (/^next\s+week$/i.test(normalized)) {
    return formatDateInTimezone(addDays(now, 7), timezone);
  }

  if (/^next\s+month$/i.test(normalized)) {
    const parts = getZonedParts(now, timezone);
    const next = withNextMonth(parts.year, parts.month);
    return toIsoDate(next.year, next.month, 1);
  }

  if (/^next\s+year$/i.test(normalized)) {
    const parts = getZonedParts(now, timezone);
    return toIsoDate(parts.year + 1, 1, 1);
  }

  const parsed = chrono.parse(normalized, now, { forwardDate: true });
  const best = parsed.find((entry) => entry.start.isCertain('day') || entry.start.isCertain('weekday') || entry.start.isCertain('month'));
  if (!best) {
    return null;
  }

  return toIsoDate(
    best.start.get('year') ?? getZonedParts(best.start.date(), timezone).year,
    best.start.get('month') ?? getZonedParts(best.start.date(), timezone).month,
    best.start.get('day') ?? getZonedParts(best.start.date(), timezone).day,
  );
};

export const resolveRecurringReminderAt = (
  recurrence: ReminderRecurrence,
  searchDateIso: string,
  searchTimeMinutes: number,
  referenceDateIso: string,
  hour: number,
  minute: number,
  anchorDayOfMonth: number | null,
  anchorMonthDay: { month: number; day: number } | null,
): string | null => {
  if (recurrence.frequency === 'daily') {
    const targetMinutes = hour * 60 + minute;
    const delta = targetMinutes > searchTimeMinutes ? 0 : recurrence.interval;
    return withIsoDateTime(addDaysToIsoDate(searchDateIso, delta), hour, minute);
  }

  if (recurrence.frequency === 'weekly') {
    if (!recurrence.days || recurrence.days.length === 0) {
      return null;
    }
    const searchWeekday = weekdayOfIsoDate(searchDateIso);
    const targetMinutes = hour * 60 + minute;
    const occurrences = recurrence.days.map((day) => {
      const targetWeekday = WEEKDAY_MAP[day];
      let delta = (targetWeekday - searchWeekday + 7) % 7;
      if (delta === 0 && targetMinutes <= searchTimeMinutes) {
        delta = 7;
      }
      return withIsoDateTime(addDaysToIsoDate(searchDateIso, delta), hour, minute);
    });
    occurrences.sort();
    return occurrences[0] || null;
  }

  if (recurrence.frequency === 'monthly') {
    if (anchorDayOfMonth) {
      return monthlyOccurrenceFromTemplate(searchDateIso, searchTimeMinutes, anchorDayOfMonth, hour, minute);
    }
    const template = parseIsoDateParts(referenceDateIso);
    return monthlyOccurrenceFromTemplate(searchDateIso, searchTimeMinutes, template.day, hour, minute);
  }

  if (recurrence.frequency === 'yearly') {
    if (anchorMonthDay) {
      return yearlyOccurrenceFromTemplate(
        searchDateIso,
        searchTimeMinutes,
        anchorMonthDay.month,
        anchorMonthDay.day,
        hour,
        minute,
      );
    }
    const template = parseIsoDateParts(referenceDateIso);
    return yearlyOccurrenceFromTemplate(searchDateIso, searchTimeMinutes, template.month, template.day, hour, minute);
  }

  return null;
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
