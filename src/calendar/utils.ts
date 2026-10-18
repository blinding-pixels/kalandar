import { DEFAULT_LOCALE, DEFAULT_TIMEZONE, TITLE_GLUE_WORDS } from './constants.ts';
import type {
  ConfidenceFields,
  DebugStep,
  EventParseResult,
  FieldSpan,
  ParseContext,
  ParseOptions,
  PassMatch,
  SpanFields,
} from './types.ts';

const emptyConfidence = (): ConfidenceFields => ({
  title: 0,
  date: 0,
  time: 0,
  end_time: 0,
  duration_minutes: 0,
  location: 0,
  recurrence: 0,
  recurrence_start: 0,
  recurrence_end: 0,
  recurrence_exception: 0,
  alerts: 0,
  attendees: 0,
  contact_suggestions: 0,
});

const emptySpans = (): SpanFields => ({
  title: [],
  date: [],
  time: [],
  end_time: [],
  duration_minutes: [],
  location: [],
  recurrence: [],
  recurrence_start: [],
  recurrence_end: [],
  recurrence_exception: [],
  alerts: [],
  attendees: [],
  contact_suggestions: [],
});

const weekdayMap: Record<string, number> = {
  sun: 0,
  sunday: 0,
  mon: 1,
  monday: 1,
  tue: 2,
  tues: 2,
  tuesday: 2,
  wed: 3,
  wednesday: 3,
  thu: 4,
  thur: 4,
  thurs: 4,
  thursday: 4,
  fri: 5,
  friday: 5,
  sat: 6,
  saturday: 6,
};

export interface ZonedDateTimeParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  weekday: number;
}

const formatterCache = new Map<string, Intl.DateTimeFormat>();

const getFormatter = (
  locale: string,
  timezone: string,
  options: Omit<Intl.DateTimeFormatOptions, 'timeZone'>,
): Intl.DateTimeFormat => {
  const key = `${locale}|${timezone}|${JSON.stringify(options)}`;
  const cached = formatterCache.get(key);
  if (cached) {
    return cached;
  }

  const formatter = new Intl.DateTimeFormat(locale, {
    ...options,
    timeZone: timezone,
  });
  formatterCache.set(key, formatter);
  return formatter;
};

export const normalizeWhitespace = (input: string): string => input.replace(/\s+/g, ' ').trim();

export const clampConfidence = (value: number): number => {
  if (Number.isNaN(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
};

export const detectSystemTimezone = (): string => {
  try {
    const detected = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (detected) {
      return detected;
    }
  } catch {
    // Ignore and fall back.
  }
  return DEFAULT_TIMEZONE;
};

export const detectSystemLocale = (): string => {
  try {
    const detected = Intl.DateTimeFormat().resolvedOptions().locale;
    if (detected) {
      return detected;
    }
  } catch {
    // Ignore and fall back.
  }
  return DEFAULT_LOCALE;
};

export function parseReferenceDate(value: string | Date | undefined): Date {
  if (value instanceof Date) {
    return value;
  }
  if (typeof value === 'string') {
    const parsed = new Date(value);
    if (!Number.isNaN(parsed.valueOf())) {
      return parsed;
    }
  }
  return new Date();
}

export const getZonedDateTimeParts = (date: Date, timezone: string, locale = 'en-US'): ZonedDateTimeParts => {
  const formatter = getFormatter(locale, timezone, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    weekday: 'short',
  });

  const parts = formatter.formatToParts(date);
  const year = Number(parts.find((part) => part.type === 'year')?.value || NaN);
  const month = Number(parts.find((part) => part.type === 'month')?.value || NaN);
  const day = Number(parts.find((part) => part.type === 'day')?.value || NaN);
  const hour = Number(parts.find((part) => part.type === 'hour')?.value || NaN);
  const minute = Number(parts.find((part) => part.type === 'minute')?.value || NaN);
  const weekdayToken =
    getFormatter('en-US', timezone, {
      weekday: 'short',
    })
      .formatToParts(date)
      .find((part) => part.type === 'weekday')
      ?.value?.toLowerCase() || '';
  const weekday = weekdayMap[weekdayToken] ?? weekdayMap[weekdayToken.slice(0, 3)] ?? NaN;

  if ([year, month, day, hour, minute, weekday].some((value) => Number.isNaN(value))) {
    throw new Error(`Unable to format zoned date parts for timezone ${timezone}`);
  }

  return {
    year,
    month,
    day,
    hour,
    minute,
    weekday,
  };
};

export const toIsoDate = (year: number, month: number, day: number): string => {
  const safeMonth = String(month).padStart(2, '0');
  const safeDay = String(day).padStart(2, '0');
  return `${String(year).padStart(4, '0')}-${safeMonth}-${safeDay}`;
};

export const isValidCalendarDate = (year: number, month: number, day: number): boolean => {
  if (!Number.isInteger(year) || !Number.isInteger(month) || !Number.isInteger(day)) {
    return false;
  }
  if (month < 1 || month > 12 || day < 1 || day > 31) {
    return false;
  }
  const utc = new Date(Date.UTC(year, month - 1, day));
  return utc.getUTCFullYear() === year && utc.getUTCMonth() === month - 1 && utc.getUTCDate() === day;
};

export const normalizeWeekdayToken = (text: string): string | null => {
  const normalized = (text || '').trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  if (/^mon/.test(normalized)) return 'monday';
  if (/^tue/.test(normalized)) return 'tuesday';
  if (/^wed/.test(normalized)) return 'wednesday';
  if (/^thu/.test(normalized)) return 'thursday';
  if (/^fri/.test(normalized)) return 'friday';
  if (/^sat/.test(normalized)) return 'saturday';
  if (/^sun/.test(normalized)) return 'sunday';
  return null;
};

export const createParseContext = (input: string, opts?: ParseOptions): ParseContext => {
  const now = opts?.now ? parseReferenceDate(opts.now) : new Date();
  const locale = opts?.locale || detectSystemLocale();
  const timezone = opts?.timezone || detectSystemTimezone();

  const cleanedInput = normalizeWhitespace(input || '');
  const result: EventParseResult = {
    fields: {
      title: null,
      date: null,
      time: null,
      end_time: null,
      duration_minutes: null,
      location: null,
      recurrence: null,
      recurrence_start: null,
      recurrence_end: null,
      recurrence_exception: [],
      alerts: null,
      attendees: null,
      contact_suggestions: null,
    },
    meta: {
      locale,
      timezone,
      confidence: emptyConfidence(),
      spans: emptySpans(),
      cleanedInput,
      maskedInput: cleanedInput,
      debugSteps: [],
    },
    warnings: null,
    schedule: null,
  };

  return {
    rawInput: cleanedInput,
    cleanedInput,
    maskedInput: cleanedInput,
    now,
    options: {
      locale,
      timezone,
      debug: Boolean(opts?.debug),
    },
    result,
  };
};

export const createFieldSpan = (ctx: ParseContext, match: PassMatch): FieldSpan => ({
  start: match.start,
  end: match.end,
  text: match.text,
  ruleId: match.ruleId,
  confidence: clampConfidence(match.confidence),
});

export const addDebugStep = (ctx: ParseContext, step: Omit<DebugStep, 'confidence'> & { confidence?: number }): void => {
  const confidence = clampConfidence(step.confidence ?? 0);
  ctx.result.meta.debugSteps.push({ ...step, confidence });
};

export const setFieldConfidence = (ctx: ParseContext, field: keyof ConfidenceFields, confidence: number): void => {
  const safe = clampConfidence(confidence);
  if (safe > ctx.result.meta.confidence[field]) {
    ctx.result.meta.confidence[field] = safe;
  }
};

export const addFieldSpan = (ctx: ParseContext, field: keyof SpanFields, match: PassMatch): void => {
  ctx.result.meta.spans[field].push(createFieldSpan(ctx, match));
};

export const maskSpan = (ctx: ParseContext, start: number, end: number): void => {
  if (end <= start || start < 0 || end > ctx.maskedInput.length) {
    return;
  }

  const before = ctx.maskedInput.slice(0, start);
  const middle = ' '.repeat(end - start);
  const after = ctx.maskedInput.slice(end);
  ctx.maskedInput = `${before}${middle}${after}`;
  ctx.result.meta.maskedInput = ctx.maskedInput;
};

export const maskMatch = (ctx: ParseContext, match: PassMatch): void => {
  maskSpan(ctx, match.start, match.end);
};

export const maskMatches = (ctx: ParseContext, matches: PassMatch[]): void => {
  for (const match of matches) {
    maskMatch(ctx, match);
  }
};

export const isSpanAvailable = (ctx: ParseContext, start: number, end: number): boolean => {
  if (end <= start || start < 0 || end > ctx.maskedInput.length) {
    return false;
  }
  return /\S/.test(ctx.maskedInput.slice(start, end));
};

export const stripEdgeGlueWords = (input: string): string => {
  const tokens = input.split(/\s+/).filter(Boolean);
  while (tokens.length > 0 && TITLE_GLUE_WORDS.has(tokens[0].toLowerCase())) {
    tokens.shift();
  }
  while (tokens.length > 0 && TITLE_GLUE_WORDS.has(tokens[tokens.length - 1].toLowerCase())) {
    tokens.pop();
  }
  return tokens.join(' ');
};

export const smartTitleCase = (input: string): string =>
  input
    .split(/\s+/)
    .map((token) => {
      if (!token) {
        return token;
      }
      if (token.toUpperCase() === token && /[A-Z]/.test(token)) {
        return token;
      }
      return token.charAt(0).toUpperCase() + token.slice(1);
    })
    .join(' ');

export const hasStructuredFields = (result: EventParseResult): boolean => {
  const fields = result.fields;
  return Boolean(
    fields.date ||
      fields.time ||
      fields.end_time ||
      fields.duration_minutes ||
      fields.location ||
      fields.recurrence ||
      fields.recurrence_start ||
      fields.recurrence_end ||
      fields.recurrence_exception?.length ||
      fields.alerts?.length ||
      fields.attendees?.length ||
      fields.contact_suggestions?.length,
  );
};
