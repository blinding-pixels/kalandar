import type {
  RecurrenceExceptionAction,
  RecurrenceExceptionCondition,
  RecurrenceExceptionOrdinal,
  RecurrenceExceptionShiftDirection,
  RecurrenceExceptionWeekday,
  StructuredRecurrenceException,
} from './types.ts';
import { getZonedDateTimeParts, isValidCalendarDate, normalizeWeekdayToken, toIsoDate } from './utils.ts';

const WEEKDAY_TOKEN =
  '(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';
const MONTH_TOKEN =
  '(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sep|sept|october|oct|november|nov|december|dec)';
const ISO_DATE_REGEX = /\b(\d{4})-(\d{2})-(\d{2})\b/;
const SLASH_DATE_REGEX = /\b(\d{1,2})[/-](\d{1,2})(?:[/-](\d{2,4}))?\b/;
const MONTH_NAME_DATE_REGEX = new RegExp(`\\b${MONTH_TOKEN}\\s+(\\d{1,2})(?:st|nd|rd|th)?(?:,?\\s+(\\d{4}))?\\b`, 'i');
const HOLIDAY_REGEX = /\bholidays?\b/i;
const WEEKEND_REGEX = /\bweekends?\b/i;
const REPLACE_REGEX = /\b(?:replace|replaced)\s+with\s+(.+?)$/i;
const SHIFT_REGEX = new RegExp(
  `\\b(?:then\\s+)?(?:move|shift)?(?:\\s+to)?\\s*(?:the\\s+)?(following|next|preceding|previous|prior)\\s+${WEEKDAY_TOKEN}\\b`,
  'i',
);
const ORDINAL_PATTERNS: RegExp[] = [
  /\bskip\s+the\s+(first|second|third|last)(?:\s+(?:one|occurrence|instance))?\b/i,
  /\bbut\s+not\s+the\s+(first|second|third|last)\b/i,
  /\bnot\s+the\s+(first|second|third|last)\b/i,
  /\bthe\s+(first|second|third|last)\s+(?:one|occurrence|instance)\b/i,
  new RegExp(`\\bthe\\s+(first|second|third|last)\\s+${WEEKDAY_TOKEN}\\s+of\\s+(?:the\\s+|every\\s+)?month\\b`, 'i'),
];

const MONTHS: Record<string, number> = {
  jan: 1,
  january: 1,
  feb: 2,
  february: 2,
  mar: 3,
  march: 3,
  apr: 4,
  april: 4,
  may: 5,
  jun: 6,
  june: 6,
  jul: 7,
  july: 7,
  aug: 8,
  august: 8,
  sep: 9,
  sept: 9,
  september: 9,
  oct: 10,
  october: 10,
  nov: 11,
  november: 11,
  dec: 12,
  december: 12,
};

export interface ClassifyRecurrenceExceptionOptions {
  now?: Date;
  timezone?: string;
}

export interface RecurrenceExceptionResolution {
  matches: boolean;
  meta?: unknown;
  unresolvable?: boolean;
}

export type RecurrenceExceptionResolverResult = boolean | Omit<RecurrenceExceptionResolution, 'unresolvable'>;
export type RecurrenceExceptionResolver = (date: Date) => RecurrenceExceptionResolverResult;

const resolverRegistry = new Map<string, RecurrenceExceptionResolver>();

const normalizeReferenceDate = (now?: Date): Date => {
  if (now instanceof Date && !Number.isNaN(now.valueOf())) {
    return now;
  }
  return new Date();
};

const normalizeReferenceYear = (now: Date, timezone: string): number =>
  getZonedDateTimeParts(now, timezone, 'en-US').year;

const normalizeOrdinal = (value: string): RecurrenceExceptionOrdinal | null => {
  const lowered = value.trim().toLowerCase();
  if (lowered === 'first') return 'first';
  if (lowered === 'second') return 'second';
  if (lowered === 'third') return 'third';
  if (lowered === 'last') return 'last';
  return null;
};

const normalizeShiftDirection = (value: string): RecurrenceExceptionShiftDirection => {
  const lowered = value.trim().toLowerCase();
  return lowered === 'preceding' || lowered === 'previous' || lowered === 'prior' ? 'preceding' : 'following';
};

const normalizeIsoDateYear = (
  month: number,
  day: number,
  explicitYear: number | null,
  referenceDate: Date,
  timezone: string,
): string | null => {
  const referenceParts = getZonedDateTimeParts(referenceDate, timezone, 'en-US');
  let year = explicitYear ?? normalizeReferenceYear(referenceDate, timezone);
  if (!isValidCalendarDate(year, month, day)) {
    return null;
  }

  if (explicitYear === null) {
    const candidate = toIsoDate(year, month, day);
    const reference = toIsoDate(referenceParts.year, referenceParts.month, referenceParts.day);
    if (candidate < reference && isValidCalendarDate(year + 1, month, day)) {
      year += 1;
    }
  }

  return toIsoDate(year, month, day);
};

const extractSpecificDate = (raw: string, options?: ClassifyRecurrenceExceptionOptions): string | null => {
  const timezone = options?.timezone || 'UTC';
  const referenceDate = normalizeReferenceDate(options?.now);

  const isoMatch = raw.match(ISO_DATE_REGEX);
  if (isoMatch) {
    const year = Number(isoMatch[1]);
    const month = Number(isoMatch[2]);
    const day = Number(isoMatch[3]);
    return isValidCalendarDate(year, month, day) ? toIsoDate(year, month, day) : null;
  }

  const slashMatch = raw.match(SLASH_DATE_REGEX);
  if (slashMatch) {
    const month = Number(slashMatch[1]);
    const day = Number(slashMatch[2]);
    const explicitYear = slashMatch[3]
      ? slashMatch[3].length === 2
        ? 2000 + Number(slashMatch[3])
        : Number(slashMatch[3])
      : null;
    return normalizeIsoDateYear(month, day, explicitYear, referenceDate, timezone);
  }

  const monthNameMatch = raw.match(MONTH_NAME_DATE_REGEX);
  if (!monthNameMatch) {
    return null;
  }

  const monthToken = monthNameMatch[1]?.toLowerCase() || '';
  const month = MONTHS[monthToken];
  const day = Number(monthNameMatch[2]);
  const explicitYear = monthNameMatch[3] ? Number(monthNameMatch[3]) : null;

  if (!month || !Number.isInteger(day)) {
    return null;
  }

  return normalizeIsoDateYear(month, day, explicitYear, referenceDate, timezone);
};

export const classifyRecurrenceExceptionCondition = (
  raw: string,
  options?: ClassifyRecurrenceExceptionOptions,
): RecurrenceExceptionCondition | null => {
  if (HOLIDAY_REGEX.test(raw)) {
    return { type: 'holiday' };
  }

  if (WEEKEND_REGEX.test(raw)) {
    return { type: 'weekend' };
  }

  for (const pattern of ORDINAL_PATTERNS) {
    const match = raw.match(pattern);
    const which = match?.[1] ? normalizeOrdinal(match[1]) : null;
    if (which) {
      return {
        type: 'ordinal',
        which,
      };
    }
  }

  const date = extractSpecificDate(raw, options);
  if (date) {
    return {
      type: 'specific_date',
      date,
    };
  }

  return null;
};

export const classifyRecurrenceExceptionAction = (raw: string): RecurrenceExceptionAction | null => {
  const shiftMatch = raw.match(SHIFT_REGEX);
  if (shiftMatch) {
    const direction = normalizeShiftDirection(shiftMatch[1] || '');
    const weekday = normalizeWeekdayToken(shiftMatch[2] || '');
    if (weekday) {
      return {
        type: 'shift',
        to: weekday as RecurrenceExceptionWeekday,
        direction,
      };
    }
  }

  const replaceMatch = raw.match(REPLACE_REGEX);
  if (replaceMatch?.[1]) {
    const replacement = replaceMatch[1].trim().replace(/[.,;]+$/g, '');
    if (replacement) {
      return {
        type: 'replace',
        with: replacement,
      };
    }
  }

  if (/\bskip\b/i.test(raw) || /\bbut\s+not\b/i.test(raw)) {
    return { type: 'skip' };
  }

  return null;
};

export const classifyRecurrenceException = (
  raw: string,
  options?: ClassifyRecurrenceExceptionOptions,
): StructuredRecurrenceException => {
  const condition = classifyRecurrenceExceptionCondition(raw, options);
  let action = classifyRecurrenceExceptionAction(raw);

  if (!action && condition && /^(?:unless|except)\b/i.test(raw.trim())) {
    action = { type: 'skip' };
  }

  if (!condition && !action) {
    return {
      raw,
      condition: null,
      action: null,
    };
  }

  return {
    raw,
    condition,
    action,
  };
};

const normalizeResolverResult = (value: RecurrenceExceptionResolverResult): RecurrenceExceptionResolution => {
  if (typeof value === 'boolean') {
    return { matches: value };
  }
  return {
    matches: Boolean(value.matches),
    meta: value.meta,
  };
};

const isoDateFromUtcDate = (date: Date): string => {
  const year = date.getUTCFullYear();
  const month = date.getUTCMonth() + 1;
  const day = date.getUTCDate();
  return toIsoDate(year, month, day);
};

const weekdayFromUtcDate = (date: Date): number => date.getUTCDay();

const ordinalFromUtcDate = (date: Date): RecurrenceExceptionOrdinal | null => {
  const dayOfMonth = date.getUTCDate();
  const occurrence = Math.floor((dayOfMonth - 1) / 7) + 1;
  if (occurrence === 1) return 'first';
  if (occurrence === 2) return 'second';
  if (occurrence === 3) return 'third';

  const weekday = date.getUTCDay();
  const lastSameWeekday = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 0));
  while (lastSameWeekday.getUTCDay() !== weekday) {
    lastSameWeekday.setUTCDate(lastSameWeekday.getUTCDate() - 1);
  }

  if (lastSameWeekday.getUTCDate() === dayOfMonth) {
    return 'last';
  }

  return null;
};

const resolveBuiltInCondition = (
  condition: RecurrenceExceptionCondition,
  date: Date,
): RecurrenceExceptionResolution | null => {
  if (condition.type === 'weekend') {
    return { matches: weekdayFromUtcDate(date) === 0 || weekdayFromUtcDate(date) === 6 };
  }

  if (condition.type === 'specific_date') {
    return { matches: isoDateFromUtcDate(date) === condition.date };
  }

  if (condition.type === 'ordinal') {
    return {
      matches: ordinalFromUtcDate(date) === condition.which,
      meta: {
        actual: ordinalFromUtcDate(date),
      },
    };
  }

  return null;
};

export const configureRecurrenceExceptionResolvers = (
  resolvers: Record<string, RecurrenceExceptionResolver | null | undefined>,
): void => {
  for (const [conditionType, resolver] of Object.entries(resolvers)) {
    if (typeof resolver === 'function') {
      resolverRegistry.set(conditionType, resolver);
      continue;
    }
    resolverRegistry.delete(conditionType);
  }
};

export const registerRecurrenceExceptionResolver = (
  conditionType: string,
  resolver: RecurrenceExceptionResolver,
): void => {
  resolverRegistry.set(conditionType, resolver);
};

export const getRecurrenceExceptionResolver = (
  conditionType: string,
): RecurrenceExceptionResolver | null => resolverRegistry.get(conditionType) || null;

export const resetRecurrenceExceptionResolvers = (): void => {
  resolverRegistry.clear();
};

export const resolveRecurrenceExceptionCondition = (
  condition: RecurrenceExceptionCondition | null,
  date: Date,
): RecurrenceExceptionResolution => {
  if (!condition) {
    return { matches: false };
  }

  const builtInResolution = resolveBuiltInCondition(condition, date);
  if (builtInResolution) {
    return builtInResolution;
  }

  const resolver = getRecurrenceExceptionResolver(condition.type);
  if (!resolver) {
    return {
      matches: false,
      unresolvable: true,
    };
  }

  return normalizeResolverResult(resolver(date));
};
