import { DEFAULT_LOCALE, DEFAULT_TIMEZONE, WEEKDAY_INDEX_BY_TOKEN } from './constants.ts';

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

export const parseReferenceDate = (value: Date | string | undefined): Date => {
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
};

export const detectSystemTimezone = (): string => {
  try {
    const detected = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (detected) {
      return detected;
    }
  } catch {
    // ignore and use fallback
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
    // ignore and use fallback
  }
  return DEFAULT_LOCALE;
};

export interface ZonedDateParts {
  year: number;
  month: number;
  day: number;
  weekday: number;
}

export interface ZonedDateTimeParts extends ZonedDateParts {
  hour: number;
  minute: number;
}

export const getZonedDateParts = (date: Date, timezone: string, locale = 'en-US'): ZonedDateParts => {
  const formatter = getFormatter(locale, timezone, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    weekday: 'short',
  });
  const parts = formatter.formatToParts(date);
  const year = Number(parts.find((part) => part.type === 'year')?.value || NaN);
  const month = Number(parts.find((part) => part.type === 'month')?.value || NaN);
  const day = Number(parts.find((part) => part.type === 'day')?.value || NaN);
  const weekdayToken =
    getFormatter('en-US', timezone, {
      weekday: 'short',
    })
      .formatToParts(date)
      .find((part) => part.type === 'weekday')
      ?.value?.toLowerCase() || '';
  const weekday = WEEKDAY_INDEX_BY_TOKEN[weekdayToken] ?? WEEKDAY_INDEX_BY_TOKEN[weekdayToken.slice(0, 3)] ?? NaN;

  if ([year, month, day, weekday].some((value) => Number.isNaN(value))) {
    throw new Error(`Unable to format zoned date parts for timezone ${timezone}`);
  }

  return {
    year,
    month,
    day,
    weekday,
  };
};

export const getZonedParts = (date: Date, timezone: string, locale = 'en-US'): ZonedDateTimeParts => {
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
  const weekday = WEEKDAY_INDEX_BY_TOKEN[weekdayToken] ?? WEEKDAY_INDEX_BY_TOKEN[weekdayToken.slice(0, 3)] ?? NaN;

  if ([year, month, day, hour, minute, weekday].some((value) => Number.isNaN(value))) {
    throw new Error(`Unable to format zoned datetime parts for timezone ${timezone}`);
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

export const toIsoDate = (year: number, month: number, day: number): string =>
  `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;

export const formatDateInTimezone = (date: Date, timezone: string): string => {
  const parts = getZonedDateParts(date, timezone);
  return toIsoDate(parts.year, parts.month, parts.day);
};

export const formatDateTimeInTimezone = (date: Date, timezone: string): string => {
  const parts = getZonedParts(date, timezone);
  return `${toIsoDate(parts.year, parts.month, parts.day)}T${String(parts.hour).padStart(2, '0')}:${String(parts.minute).padStart(2, '0')}:00`;
};

export const addDays = (date: Date, days: number): Date => {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
};

export const addMinutes = (date: Date, minutes: number): Date => new Date(date.getTime() + minutes * 60000);
