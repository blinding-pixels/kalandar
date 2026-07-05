import * as chrono from 'chrono-node';
import type { ParsedComponents, ParsedResult } from 'chrono-node';
import type { ParseContext } from '../types.ts';
import {
  addDaysToIsoDate,
  addDebugStep,
  addFieldSpan,
  formatDateInTimezone,
  getZonedDateTimeParts,
  isValidCalendarDate,
  isSpanAvailable,
  maskMatch,
  replaceWithSameLength,
  setFieldConfidence,
  toIsoDate,
  toIsoTime,
} from '../utils.ts';

type WeekdayModifier = null | 'this' | 'next' | 'next-week';

type RelativeWeekdayResolution = {
  dateISO: string;
  confidence: number;
  ruleId: string;
  note: string;
};

type TimeParts = {
  hour: number;
  minute: number;
  explicit: boolean;
};

const WEEKDAY_TOKEN =
  '(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';
const DAYPART_TOKEN = '(?:morning|afternoon|evening|night|noon)';
const OPTIONAL_DAYPART = `(?:\\s+${DAYPART_TOKEN})?`;
const MONTH_TOKEN =
  '(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t|tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)';

const NEXT_WEEK_DAY_REGEX = new RegExp(`\\bnext\\s+week(?:\\s+on)?\\s+${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i');
const RELATIVE_DAY_REGEX = new RegExp(`\\b(this|next)\\s+${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i');
const BARE_DAY_REGEX = new RegExp(`\\b${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i');
const WEEKDAY_SIGNAL_REGEX = new RegExp(`\\b${WEEKDAY_TOKEN}\\b`, 'i');

const DATE_SIGNAL_REGEX =
  /\b(today|tomorrow|tmr|tonight|yesterday|next|this|end of day|eod|end of week|eow|end of month|in\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec|\d{4}-\d{2}-\d{2}|\d{1,2}[/-]\d{1,2})\b/i;

const DATE_FRAGMENT_PATTERNS: RegExp[] = [
  new RegExp(`\\bnext\\s+week(?:\\s+on)?\\s+${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i'),
  new RegExp(`\\b(?:this|next)\\s+${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i'),
  new RegExp(`\\b${WEEKDAY_TOKEN}${OPTIONAL_DAYPART}\\b`, 'i'),
  /\b(?:today|tomorrow|tmr|tonight|yesterday)\b/i,
  new RegExp(`\\b${MONTH_TOKEN}\\s+\\d{1,2}(?:,\\s*\\d{4})?\\b`, 'i'),
  /\b\d{4}-\d{2}-\d{2}\b/i,
  /\b\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?\b/i,
  /(?:^|\s)\d+\s+(?:day|days|week|weeks|month|months|year|years)\s+from\s+now\b/i,
  /(?:^|\s)in\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b/i,
];

const RANGE_TOKEN = '(?:[01]?\\d|2[0-3])(?::[0-5]\\d)?\\s*(?:am|pm)?|noon|midnight';
const EXPLICIT_TIME_RANGE_REGEX = new RegExp(`\\b(${RANGE_TOKEN})\\s*(?:-|–|to)\\s*(${RANGE_TOKEN})\\b`, 'gi');
const EXPLICIT_TIME_REGEX = /\b(?:at\s+)?((?:[01]?\d|2[0-3]):[0-5]\d\s*(?:am|pm)?|(?:[1-9]|1[0-2])\s*(?:am|pm)|noon|midnight)\b/gi;
const BARE_HOUR_REGEX = /\b(?:at\s+)?([1-9]|1[0-2])\b/gi;

const MONTH_INDEX: Record<string, number> = {
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

const monthDayRegex = new RegExp(`\\b(${MONTH_TOKEN})\\s+(\\d{1,2})(?:,\\s*(\\d{4}))?\\b`, 'gi');
const RECURRENCE_START_REGEX =
  /\b(?:starting|beginning)\s+(.+?)(?=(?:\s+\b(?:until|till|through|ending|except|remind|alert|with|every|for|at|on|from|to)\b|[,.;]|$))/i;
const EXPLICIT_DATE_TOKEN_REGEX =
  /\b(?:\d{4}-\d{2}-\d{2}|(?<!:)\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?(?!:)|(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t|tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}(?:st|nd|rd|th)?(?:,\s*\d{4})?)\b/i;
const UNSUPPORTED_RELATIVE_TIME_RANGE_REGEX = /\bfrom\s+(.+?)\s+until\s+(.+?\blater)\b/gi;
const UNSUPPORTED_APPROXIMATE_TIME_WINDOW_REGEX = /\bfrom\s+\d{1,2}(?::\d{2})?ish\s+(?:-|–|to)\s+before\s+\d{1,2}(?::\d{2})?\b/gi;
const UNSUPPORTED_SPECULATIVE_DURATION_REGEX =
  /\bmaybe\s+\d+(?:\.\d+)?\s+(?:minutes?|hours?)\s+but\s+not\s+longer\s+than\s+\d+(?:\.\d+)?\b/gi;
const UNSUPPORTED_RESCHEDULE_CLAUSE_REGEX = /\bunless\s+that\s+runs\s+too\s+late\s+then\s+start\s+earlier\b/gi;
const EXPLICIT_CLOCK_SIGNAL_REGEX = /\b(?:\d{1,2}(?::\d{2})?\s*(?:am|pm)|noon|midnight)\b/i;
const RELATIVE_DURATION_REGEX =
  /\b(?:maybe\s+|about\s+|around\s+|roughly\s+)?\d+(?:\.\d+)?\s*(?:m|min|mins|minute|minutes|h|hr|hrs|hour|hours|day|days|week|weeks|month|months|year|years)\b/i;

const endOfMonthIso = (now: Date, timezone: string): string => {
  const parts = getZonedDateTimeParts(now, timezone);
  const lastDay = new Date(Date.UTC(parts.year, parts.month, 0)).getUTCDate();
  return toIsoDate(parts.year, parts.month, lastDay);
};

const fridayOfCurrentWeekIso = (now: Date, timezone: string): string => {
  const parts = getZonedDateTimeParts(now, timezone);
  let delta = parts.weekday <= 5 ? 5 - parts.weekday : 12 - parts.weekday;
  if (parts.weekday === 5 && parts.hour >= 17) {
    delta = 7;
  }
  const todayISO = toIsoDate(parts.year, parts.month, parts.day);
  return addDaysToIsoDate(todayISO, delta);
};

const endOfDayReference = (
  now: Date,
  timezone: string,
): {
  chronoText: string;
  dateISO: string;
  timeISO: string;
} => {
  const parts = getZonedDateTimeParts(now, timezone);
  const todayISO = toIsoDate(parts.year, parts.month, parts.day);
  const useTomorrow = parts.hour >= 17;

  return {
    chronoText: useTomorrow ? 'tomorrow 5pm' : 'today 5pm',
    dateISO: useTomorrow ? addDaysToIsoDate(todayISO, 1) : todayISO,
    timeISO: '17:00',
  };
};

const weekdayIndexByToken = (token: string): number | null => {
  const normalized = token.trim().toLowerCase();
  if (/^mon/.test(normalized)) return 1;
  if (/^tue/.test(normalized)) return 2;
  if (/^wed/.test(normalized)) return 3;
  if (/^thu/.test(normalized)) return 4;
  if (/^fri/.test(normalized)) return 5;
  if (/^sat/.test(normalized)) return 6;
  if (/^sun/.test(normalized)) return 0;
  return null;
};

export const resolveRelativeWeekday = (
  token: string,
  modifier: WeekdayModifier,
  now: Date,
  timezone: string,
): RelativeWeekdayResolution | null => {
  const targetWeekday = weekdayIndexByToken(token);
  if (targetWeekday === null) {
    return null;
  }

  const zonedNow = getZonedDateTimeParts(now, timezone);
  const currentDateISO = toIsoDate(zonedNow.year, zonedNow.month, zonedNow.day);
  const currentWeekday = zonedNow.weekday;

  const currentWeekIso = (currentWeekday + 6) % 7;
  const targetWeekIso = (targetWeekday + 6) % 7;

  let deltaDays = 0;
  let confidence = 0.84;
  let ruleId = 'datetime.relative_weekday';
  let note = 'resolved relative weekday using upcoming occurrence semantics';

  if (modifier === null) {
    deltaDays = (targetWeekday - currentWeekday + 7) % 7;
    confidence = 0.84;
    ruleId = 'datetime.weekday.bare';
    note = 'bare weekday resolved to next occurrence (including today)';
  } else if (modifier === 'next') {
    deltaDays = (targetWeekday - currentWeekday + 7) % 7;
    if (deltaDays === 0) {
      deltaDays = 7;
      note = 'next weekday on same day resolved to +7 days';
    } else {
      note = 'next weekday resolved to upcoming occurrence';
    }
    confidence = 0.88;
    ruleId = 'datetime.weekday.next';
  } else if (modifier === 'this') {
    if (targetWeekIso >= currentWeekIso) {
      deltaDays = targetWeekIso - currentWeekIso;
      confidence = 0.9;
      note = 'this weekday resolved in current week (including today)';
    } else {
      deltaDays = 7 - (currentWeekIso - targetWeekIso);
      confidence = 0.62;
      note = 'weekday already passed this week; choosing upcoming occurrence';
    }
    ruleId = 'datetime.weekday.this';
  } else {
    const daysUntilNextMonday = 7 - currentWeekIso;
    deltaDays = daysUntilNextMonday + targetWeekIso;
    confidence = 0.91;
    ruleId = 'datetime.weekday.next_week';
    note = 'next week weekday resolved from next ISO week Monday';
  }

  if (deltaDays < 0) {
    deltaDays = 0;
  }

  const dateISO = addDaysToIsoDate(currentDateISO, deltaDays);
  return {
    dateISO,
    confidence,
    ruleId,
    note,
  };
};

const hasDateSignal = (text: string): boolean => DATE_SIGNAL_REGEX.test(text) || WEEKDAY_SIGNAL_REGEX.test(text);

const preprocessForChrono = (ctx: ParseContext): string => {
  const endOfDay = endOfDayReference(ctx.now, ctx.options.timezone);
  let output = ctx.maskedInput;
  output = replaceWithSameLength(output, /\bfirst\s+thing\b/gi, '9am');
  output = replaceWithSameLength(output, /\btmr\b/gi, 'tom');
  output = replaceWithSameLength(output, /\b(?:end of (?:the )?day|eod)\b/gi, endOfDay.chronoText);
  output = replaceWithSameLength(output, /\b(?:end of (?:the )?week|eow)\b/gi, fridayOfCurrentWeekIso(ctx.now, ctx.options.timezone));
  output = replaceWithSameLength(output, /\bend\s+of\s+(?:the\s+)?month\b/gi, endOfMonthIso(ctx.now, ctx.options.timezone));

  return output;
};

const dateScore = (entry: ParsedResult): number => {
  const text = entry.text || '';
  let score = 0;
  const hasCertainDatePart =
    entry.start.isCertain('day') ||
    entry.start.isCertain('month') ||
    entry.start.isCertain('year') ||
    entry.start.isCertain('weekday');

  if (entry.start.isCertain('day')) score += 4;
  if (entry.start.isCertain('month')) score += 2;
  if (entry.start.isCertain('year')) score += 1;
  if (entry.start.isCertain('weekday')) score += 1;
  if (WEEKDAY_SIGNAL_REGEX.test(text)) score += 2;
  if (hasDateSignal(text)) score += 1;
  if (/\bnext\s+week\b/i.test(text) && !WEEKDAY_SIGNAL_REGEX.test(text)) score -= 3;
  if (!hasCertainDatePart && entry.start.isCertain('hour')) score -= 5;
  if (!hasCertainDatePart && /\bfor\s+\d+\s*(?:m|min|mins|minute|minutes|h|hr|hrs|hour|hours)\b/i.test(text)) score -= 6;
  if (RELATIVE_DURATION_REGEX.test(text) && !hasDateSignal(text)) score -= 8;
  if (/\b(?:past the hour|hour later|minutes later|half an hour later|quarter of an hour later|three-quarters of an hour later)\b/i.test(text)) {
    score -= 6;
  }
  if (/\busual\b/i.test(text)) score -= 4;

  return score;
};

const timeScore = (entry: ParsedResult): number => {
  const text = entry.text || '';
  let score = 0;

  if (entry.start.isCertain('hour')) score += 3;
  if (entry.start.isCertain('minute')) score += 0.4;
  if (entry.start.isCertain('meridiem')) score += 1;
  if (/\b(?:am|pm|noon|midnight)\b/i.test(text)) score += 1;
  if (entry.end?.isCertain('hour')) score += 0.6;
  if (/\b(?:at|from)\s+\d/i.test(text)) score += 0.2;
  if (/\b\d+\s*(?:m|min|mins|minute|minutes)\b/i.test(text) && !/\b(?:am|pm)\b/i.test(text)) score -= 2;
  if (RELATIVE_DURATION_REGEX.test(text) && !/\b(?:am|pm|noon|midnight)\b/i.test(text)) score -= 6;
  if (/\b(?:past the hour|hour later|minutes later|half an hour later|quarter of an hour later|three-quarters of an hour later)\b/i.test(text)) {
    score -= 5;
  }
  if (/\busual\b/i.test(text)) score -= 4;

  return score;
};

const pickBestResult = (results: ParsedResult[], scorer: (result: ParsedResult) => number): ParsedResult | null => {
  let best: ParsedResult | null = null;
  let bestScore = Number.NEGATIVE_INFINITY;

  for (const entry of results) {
    const score = scorer(entry);
    if (score > bestScore) {
      best = entry;
      bestScore = score;
    }
  }

  return bestScore > 0 ? best : null;
};

const explicitMeridiemInText = (text: string): boolean => /\b(?:am|pm)\b/i.test(text);

const normalizeAmbiguousHour = (
  rawHour: number,
  text: string,
  companionHour: number | null,
  explicitMeridiem: boolean,
): number => {
  if (rawHour >= 13 || rawHour === 0 || rawHour === 12) {
    return rawHour;
  }
  if (explicitMeridiem || /\b(?:am|pm)\b/i.test(text)) {
    return rawHour;
  }
  if (companionHour !== null && companionHour >= 13) {
    return rawHour + 12;
  }
  if (rawHour >= 1 && rawHour <= 7) {
    return rawHour + 12;
  }
  return rawHour;
};

const minutesBetween = (startHour: number, startMinute: number, endHour: number, endMinute: number): number => {
  const start = startHour * 60 + startMinute;
  let end = endHour * 60 + endMinute;
  if (end <= start) {
    end += 24 * 60;
  }
  return end - start;
};

const findSubSpan = (entry: ParsedResult, patterns: RegExp[]): { start: number; end: number; text: string } => {
  for (const pattern of patterns) {
    pattern.lastIndex = 0;
    const match = pattern.exec(entry.text);
    if (!match || match.index === undefined) {
      continue;
    }
    return {
      start: entry.index + match.index,
      end: entry.index + match.index + match[0].length,
      text: match[0],
    };
  }

  return {
    start: entry.index,
    end: entry.index + entry.text.length,
    text: entry.text,
  };
};

const dateIsoFromComponent = (component: ParsedComponents, timezone: string): string | null => {
  const year = component.get('year');
  const month = component.get('month');
  const day = component.get('day');

  if (typeof year === 'number' && typeof month === 'number' && typeof day === 'number') {
    return toIsoDate(year, month, day);
  }

  if (component.isCertain('weekday') || component.isCertain('day') || component.isCertain('month')) {
    return formatDateInTimezone(component.date(), timezone);
  }

  return null;
};

const applyDateFromChrono = (ctx: ParseContext, entry: ParsedResult): void => {
  const dateISO = dateIsoFromComponent(entry.start, ctx.options.timezone);
  if (!dateISO) {
    return;
  }

  const span = findSubSpan(entry, DATE_FRAGMENT_PATTERNS);
  if (/^in\b/i.test(span.text.trimStart()) && span.start > 0 && ctx.rawInput[span.start - 1] === '-') {
    const fallback = /\b\d+\s+(?:day|days|week|weeks|month|months|year|years)(?:\s+from\s+now)?\b/i.exec(span.text);
    if (fallback?.index !== undefined) {
      span.start += fallback.index;
      span.end = span.start + fallback[0].length;
      span.text = fallback[0];
    }
  }
  ctx.result.fields.date = dateISO;
  const confidence = Math.min(0.95, 0.7 + dateScore(entry) * 0.05);

  addFieldSpan(ctx, 'date', {
    start: span.start,
    end: span.end,
    text: span.text,
    ruleId: 'datetime.chrono_date',
    confidence,
  });
  setFieldConfidence(ctx, 'date', confidence);
  addDebugStep(ctx, {
    pass: 'datetime',
    ruleId: 'datetime.chrono_date',
    start: span.start,
    end: span.end,
    text: span.text,
    confidence,
    note: 'date extracted by chrono parser',
  });
  maskMatch(ctx, {
    start: span.start,
    end: span.end,
    text: span.text,
    ruleId: 'datetime.mask_date',
    confidence,
  });
};

const applyTimeValue = (
  ctx: ParseContext,
  payload: {
    time: string;
    endTime: string | null;
    durationMinutes: number | null;
    start: number;
    end: number;
    text: string;
    ruleId: string;
    confidence: number;
    note: string;
  },
): void => {
  ctx.result.fields.time = payload.time;
  addFieldSpan(ctx, 'time', {
    start: payload.start,
    end: payload.end,
    text: payload.text,
    ruleId: payload.ruleId,
    confidence: payload.confidence,
  });
  setFieldConfidence(ctx, 'time', payload.confidence);

  if (payload.endTime) {
    ctx.result.fields.end_time = payload.endTime;
    addFieldSpan(ctx, 'end_time', {
      start: payload.start,
      end: payload.end,
      text: payload.text,
      ruleId: `${payload.ruleId}.end`,
      confidence: Math.max(0.7, payload.confidence - 0.05),
    });
    setFieldConfidence(ctx, 'end_time', Math.max(0.7, payload.confidence - 0.05));
  }

  if (payload.durationMinutes && payload.durationMinutes > 0 && payload.durationMinutes <= 12 * 60) {
    ctx.result.fields.duration_minutes = payload.durationMinutes;
    addFieldSpan(ctx, 'duration_minutes', {
      start: payload.start,
      end: payload.end,
      text: payload.text,
      ruleId: `${payload.ruleId}.duration`,
      confidence: Math.max(0.66, payload.confidence - 0.1),
    });
    setFieldConfidence(ctx, 'duration_minutes', Math.max(0.66, payload.confidence - 0.1));
  }

  addDebugStep(ctx, {
    pass: 'datetime',
    ruleId: payload.ruleId,
    start: payload.start,
    end: payload.end,
    text: payload.text,
    confidence: payload.confidence,
    note: payload.note,
  });
  maskMatch(ctx, {
    start: payload.start,
    end: payload.end,
    text: payload.text,
    ruleId: `${payload.ruleId}.mask`,
    confidence: payload.confidence,
  });
};

const parseClockToken = (raw: string): TimeParts | null => {
  const token = raw.trim().toLowerCase();
  if (!token) {
    return null;
  }

  if (token === 'noon') {
    return { hour: 12, minute: 0, explicit: true };
  }
  if (token === 'midnight') {
    return { hour: 0, minute: 0, explicit: true };
  }

  const ampmMatch = token.match(/^(\d{1,2})(?::([0-5]\d))?\s*(am|pm)$/i);
  if (ampmMatch) {
    const baseHour = Number(ampmMatch[1]);
    const minute = Number(ampmMatch[2] || '0');
    if (baseHour < 1 || baseHour > 12) {
      return null;
    }
    const meridiem = ampmMatch[3].toLowerCase();
    let hour = baseHour % 12;
    if (meridiem === 'pm') {
      hour += 12;
    }
    return { hour, minute, explicit: true };
  }

  const colonMatch = token.match(/^(\d{1,2}):([0-5]\d)$/);
  if (colonMatch) {
    const hour = Number(colonMatch[1]);
    const minute = Number(colonMatch[2]);
    if (hour > 23) {
      return null;
    }
    const explicit = hour >= 13 || colonMatch[1].startsWith('0');
    return { hour, minute, explicit };
  }

  const bareMatch = token.match(/^(\d{1,2})$/);
  if (bareMatch) {
    const hour = Number(bareMatch[1]);
    if (hour > 23) {
      return null;
    }
    return { hour, minute: 0, explicit: hour >= 13 };
  }

  return null;
};

const alignAmbiguousRange = (start: TimeParts, end: TimeParts): { start: TimeParts; end: TimeParts } => {
  const adjustedStart = { ...start };
  const adjustedEnd = { ...end };

  if (adjustedStart.explicit && !adjustedEnd.explicit && adjustedStart.hour >= 12 && adjustedEnd.hour <= 12) {
    if (adjustedStart.hour >= 12 && adjustedEnd.hour < 12) {
      adjustedEnd.hour += 12;
      adjustedEnd.explicit = true;
    }
  }

  if (!adjustedStart.explicit && adjustedEnd.explicit && adjustedEnd.hour >= 12 && adjustedStart.hour < 12) {
    adjustedStart.hour += 12;
    adjustedStart.explicit = true;
  }

  if (!adjustedStart.explicit && !adjustedEnd.explicit) {
    if (adjustedStart.hour <= 7 && adjustedEnd.hour <= 7) {
      adjustedStart.hour += 12;
      adjustedEnd.hour += 12;
      adjustedStart.explicit = true;
      adjustedEnd.explicit = true;
    } else {
      if (adjustedStart.hour <= 7 && adjustedStart.minute === 0) {
        adjustedStart.hour += 12;
      }
      if (adjustedEnd.hour <= 7 && adjustedEnd.minute === 0) {
        adjustedEnd.hour += 12;
      }
    }
  }

  return { start: adjustedStart, end: adjustedEnd };
};

const extractClockTokens = (text: string): TimeParts[] => {
  EXPLICIT_TIME_RANGE_REGEX.lastIndex = 0;
  const rangeMatch = EXPLICIT_TIME_RANGE_REGEX.exec(text);
  EXPLICIT_TIME_RANGE_REGEX.lastIndex = 0;
  if (rangeMatch) {
    const first = parseClockToken(rangeMatch[1] || '');
    const second = parseClockToken(rangeMatch[2] || '');
    if (first && second) {
      const firstToken = (rangeMatch[1] || '').toLowerCase();
      const secondToken = (rangeMatch[2] || '').toLowerCase();
      const explicitClockSignal =
        /[:]|am|pm|noon|midnight/.test(firstToken) || /[:]|am|pm|noon|midnight/.test(secondToken);
      if (!explicitClockSignal && (first.hour > 12 || second.hour > 12)) {
        // Likely MM-DD fragment from an ISO date, not a clock range.
      } else {
        const aligned = alignAmbiguousRange(first, second);
        return [aligned.start, aligned.end];
      }
    }
  }

  const explicitTokenRegex = /(?:[01]?\d|2[0-3]):[0-5]\d\s*(?:am|pm)?|(?:[1-9]|1[0-2])\s*(?:am|pm)|noon|midnight/i;
  const explicitMatch = explicitTokenRegex.exec(text);
  if (explicitMatch) {
    const parsed = parseClockToken(explicitMatch[0]);
    if (parsed) {
      return [parsed];
    }
  }

  const atHourMatch = /\bat\s+([1-9]|1[0-2])\b/i.exec(text);
  if (atHourMatch) {
    const parsed = parseClockToken(atHourMatch[1]);
    if (parsed) {
      return [parsed];
    }
  }

  const candidates = Array.from(text.matchAll(/\b([1-9]|1[0-2])\b/gi));
  for (let index = candidates.length - 1; index >= 0; index -= 1) {
    const candidate = candidates[index];
    const token = candidate[1] || '';
    const tokenStart = candidate.index ?? 0;
    const tokenEnd = tokenStart + token.length;
    const before = text.slice(Math.max(0, tokenStart - 5), tokenStart).toLowerCase();
    const after = text.slice(tokenEnd, tokenEnd + 12).toLowerCase();
    if (/(?:^|\s)(?:in|for|every)\s*$/.test(before)) {
      continue;
    }
    if (/^\s*(?:day|days|week|weeks|month|months|year|years)\b/.test(after)) {
      continue;
    }
    const parsed = parseClockToken(token);
    if (parsed) {
      return [parsed];
    }
  }

  return [];
};

const applyExplicitRangeFallback = (ctx: ParseContext): void => {
  EXPLICIT_TIME_RANGE_REGEX.lastIndex = 0;
  const matches = Array.from(ctx.maskedInput.matchAll(EXPLICIT_TIME_RANGE_REGEX));
  for (const match of matches) {
    const fullText = match[0];
    const start = match.index ?? -1;
    const end = start + fullText.length;
    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const nearby = ctx.rawInput.slice(Math.max(0, start - 6), Math.min(ctx.rawInput.length, end + 6));
    if (/\d{4}-\d{2}-\d{2}/.test(nearby)) {
      continue;
    }

    if (ctx.result.fields.time !== null && ctx.result.fields.end_time !== null) {
      continue;
    }

    const rawStart = parseClockToken(match[1] || '');
    const rawEnd = parseClockToken(match[2] || '');
    if (!rawStart || !rawEnd) {
      continue;
    }

    const startToken = (match[1] || '').toLowerCase();
    const endToken = (match[2] || '').toLowerCase();
    const explicitClockSignal = /[:]|am|pm|noon|midnight/.test(startToken) || /[:]|am|pm|noon|midnight/.test(endToken);
    if (!explicitClockSignal && (rawStart.hour > 12 || rawEnd.hour > 12)) {
      continue;
    }

    const aligned = alignAmbiguousRange(rawStart, rawEnd);
    const startISO = toIsoTime(aligned.start.hour, aligned.start.minute);
    const endISO = toIsoTime(aligned.end.hour, aligned.end.minute);
    const durationMinutes = minutesBetween(aligned.start.hour, aligned.start.minute, aligned.end.hour, aligned.end.minute);
    const confidence = /(am|pm|noon|midnight)/i.test(fullText) ? 0.92 : 0.82;

    applyTimeValue(ctx, {
      time: startISO,
      endTime: endISO,
      durationMinutes,
      start,
      end,
      text: fullText,
      ruleId: 'datetime.explicit_range',
      confidence,
      note: 'time range extracted from explicit range phrase',
    });
  }
};

const applyTimeFromChrono = (ctx: ParseContext, entry: ParsedResult): void => {
  const span = findSubSpan(entry, [EXPLICIT_TIME_REGEX, /\bat\s+(?:[1-9]|1[0-2])\b/i]);
  const textParts = extractClockTokens(span.text);

  const startComponent = entry.start;
  const textStart = textParts[0];
  const rawStartHour = typeof textStart?.hour === 'number' ? textStart.hour : startComponent.get('hour');
  const rawStartMinute = typeof textStart?.minute === 'number' ? textStart.minute : startComponent.get('minute') || 0;

  if (typeof rawStartHour !== 'number') {
    return;
  }

  const explicitMeridiem =
    explicitMeridiemInText(entry.text) || /\b(?:noon|midnight)\b/i.test(entry.text) || Boolean(textStart?.explicit);
  const textEnd = textParts.length > 1 ? textParts[1] : null;
  const endRawHour = typeof textEnd?.hour === 'number' ? textEnd.hour : typeof entry.end?.get('hour') === 'number' ? entry.end.get('hour') : null;
  const normalizedStartHour = normalizeAmbiguousHour(rawStartHour, entry.text, endRawHour, explicitMeridiem);
  const startTimeISO = toIsoTime(normalizedStartHour, rawStartMinute);

  let endTimeISO: string | null = null;
  let inferredDuration: number | null = null;

  if (typeof endRawHour === 'number') {
    const rawEndMinute = typeof textEnd?.minute === 'number' ? textEnd.minute : entry.end?.get('minute') || 0;
    const normalizedEndHour = normalizeAmbiguousHour(endRawHour, entry.text, normalizedStartHour, explicitMeridiem);
    endTimeISO = toIsoTime(normalizedEndHour, rawEndMinute);
    inferredDuration = minutesBetween(normalizedStartHour, rawStartMinute, normalizedEndHour, rawEndMinute);
  }

  const confidence = explicitMeridiem ? 0.92 : 0.78;

  applyTimeValue(ctx, {
    time: startTimeISO,
    endTime: endTimeISO,
    durationMinutes: inferredDuration,
    start: span.start,
    end: span.end,
    text: span.text,
    ruleId: 'datetime.chrono_time',
    confidence,
    note: 'time extracted by chrono parser',
  });
};

const applyExplicitSingleTimeFallback = (ctx: ParseContext): void => {
  if (ctx.result.fields.time !== null) {
    return;
  }

  EXPLICIT_TIME_REGEX.lastIndex = 0;
  const matches = Array.from(ctx.maskedInput.matchAll(EXPLICIT_TIME_REGEX));
  for (const match of matches) {
    const fullText = match[0];
    const token = match[1] || '';
    const start = (match.index ?? -1) + fullText.indexOf(token);
    const end = start + token.length;

    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const trailing = ctx.maskedInput.slice(end, end + 6);
    if (/^\s*(?:-|–|to)\b/i.test(trailing)) {
      continue;
    }

    const parsed = parseClockToken(token);
    if (!parsed) {
      continue;
    }

    const normalizedHour =
      parsed.explicit || parsed.hour >= 8
        ? parsed.hour
        : normalizeAmbiguousHour(parsed.hour, token, null, explicitMeridiemInText(token));
    const confidence = /(am|pm|noon|midnight)/i.test(token) || token.includes(':') ? 0.88 : 0.72;

    applyTimeValue(ctx, {
      time: toIsoTime(normalizedHour, parsed.minute),
      endTime: null,
      durationMinutes: null,
      start,
      end,
      text: token,
      ruleId: 'datetime.explicit_time',
      confidence,
      note: 'single time extracted from explicit time token',
    });
    return;
  }
};

const applyBareHourFallback = (ctx: ParseContext): void => {
  if (ctx.result.fields.time !== null) {
    return;
  }

  const hasDateContext = ctx.result.fields.date !== null || hasDateSignal(ctx.rawInput);
  if (!hasDateContext) {
    return;
  }

  BARE_HOUR_REGEX.lastIndex = 0;
  const matches = Array.from(ctx.maskedInput.matchAll(BARE_HOUR_REGEX));
  for (let index = matches.length - 1; index >= 0; index -= 1) {
    const match = matches[index];
    const token = match[1] || '';
    const fullText = match[0];
    const fullStart = match.index ?? -1;
    const start = fullStart + fullText.indexOf(token);
    const end = start + token.length;

    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const prev = ctx.rawInput.slice(Math.max(0, start - 6), start).toLowerCase();
    const next = ctx.rawInput.slice(end, end + 10).toLowerCase();
    if (/\d\s*:\s*$/.test(prev) || /^\s*:\s*\d/.test(next)) {
      continue;
    }
    if (/(?:^|\s)(?:for|in|every|x)\s*$/.test(prev)) {
      continue;
    }
    if (/^\s*(?:m|min|mins|minute|minutes|h|hr|hrs|hour|hours|day|days|week|weeks|month|months|year|years)\b/.test(next)) {
      continue;
    }

    const hour = Number(token);
    if (!Number.isInteger(hour) || hour < 1 || hour > 12) {
      continue;
    }

    const normalizedHour = normalizeAmbiguousHour(hour, token, null, false);
    applyTimeValue(ctx, {
      time: toIsoTime(normalizedHour, 0),
      endTime: null,
      durationMinutes: null,
      start,
      end,
      text: token,
      ruleId: 'datetime.bare_hour',
      confidence: 0.64,
      note: 'fallback bare hour interpreted as event time',
    });
    return;
  }
};

const applyRelativeWeekdayOverride = (ctx: ParseContext): void => {
  const hasRecurrence = Boolean(ctx.result.fields.recurrence);
  const hasStarting = /\b(?:starting|from)\b/i.test(ctx.rawInput);
  const source = ctx.rawInput;
  const hasExplicitDateToken = EXPLICIT_DATE_TOKEN_REGEX.test(source);
  if (ctx.result.fields.date !== null && hasExplicitDateToken) {
    return;
  }

  const nextWeek = NEXT_WEEK_DAY_REGEX.exec(source);
  if (nextWeek?.index !== undefined && (!hasRecurrence || hasStarting)) {
    const resolved = resolveRelativeWeekday(nextWeek[1], 'next-week', ctx.now, ctx.options.timezone);
    if (resolved) {
      ctx.result.fields.date = resolved.dateISO;
      addFieldSpan(ctx, 'date', {
        start: nextWeek.index,
        end: nextWeek.index + nextWeek[0].length,
        text: nextWeek[0],
        ruleId: resolved.ruleId,
        confidence: resolved.confidence,
      });
      ctx.result.meta.confidence.date = resolved.confidence;
      addDebugStep(ctx, {
        pass: 'datetime',
        ruleId: resolved.ruleId,
        start: nextWeek.index,
        end: nextWeek.index + nextWeek[0].length,
        text: nextWeek[0],
        confidence: resolved.confidence,
        note: resolved.note,
      });
      maskMatch(ctx, {
        start: nextWeek.index,
        end: nextWeek.index + nextWeek[0].length,
        text: nextWeek[0],
        ruleId: `${resolved.ruleId}.mask`,
        confidence: resolved.confidence,
      });
      return;
    }
  }

  const relative = RELATIVE_DAY_REGEX.exec(source);
  if (relative?.index !== undefined && (!hasRecurrence || hasStarting)) {
    const modifier = (relative[1].toLowerCase() as 'this' | 'next') || null;
    const resolved = resolveRelativeWeekday(relative[2], modifier, ctx.now, ctx.options.timezone);
    if (resolved) {
      ctx.result.fields.date = resolved.dateISO;
      addFieldSpan(ctx, 'date', {
        start: relative.index,
        end: relative.index + relative[0].length,
        text: relative[0],
        ruleId: resolved.ruleId,
        confidence: resolved.confidence,
      });
      ctx.result.meta.confidence.date = resolved.confidence;
      addDebugStep(ctx, {
        pass: 'datetime',
        ruleId: resolved.ruleId,
        start: relative.index,
        end: relative.index + relative[0].length,
        text: relative[0],
        confidence: resolved.confidence,
        note: resolved.note,
      });
      maskMatch(ctx, {
        start: relative.index,
        end: relative.index + relative[0].length,
        text: relative[0],
        ruleId: `${resolved.ruleId}.mask`,
        confidence: resolved.confidence,
      });
      return;
    }
  }

  const bare = BARE_DAY_REGEX.exec(source);
  if (bare?.index !== undefined && !hasRecurrence) {
    const resolved = resolveRelativeWeekday(bare[1], null, ctx.now, ctx.options.timezone);
    if (resolved) {
      ctx.result.fields.date = resolved.dateISO;
      addFieldSpan(ctx, 'date', {
        start: bare.index,
        end: bare.index + bare[0].length,
        text: bare[0],
        ruleId: resolved.ruleId,
        confidence: resolved.confidence,
      });
      ctx.result.meta.confidence.date = resolved.confidence;
      addDebugStep(ctx, {
        pass: 'datetime',
        ruleId: resolved.ruleId,
        start: bare.index,
        end: bare.index + bare[0].length,
        text: bare[0],
        confidence: resolved.confidence,
        note: resolved.note,
      });
      maskMatch(ctx, {
        start: bare.index,
        end: bare.index + bare[0].length,
        text: bare[0],
        ruleId: `${resolved.ruleId}.mask`,
        confidence: resolved.confidence,
      });
    }
  }
};

const applyMonthDayFallback = (ctx: ParseContext): void => {
  if (ctx.result.fields.date !== null) {
    return;
  }

  monthDayRegex.lastIndex = 0;
  const matches = Array.from(ctx.maskedInput.matchAll(monthDayRegex));
  if (matches.length === 0) {
    return;
  }

  const localNow = getZonedDateTimeParts(ctx.now, ctx.options.timezone);
  const todayISO = toIsoDate(localNow.year, localNow.month, localNow.day);

  for (const match of matches) {
    const monthToken = (match[1] || '').toLowerCase();
    const month = MONTH_INDEX[monthToken];
    const day = Number(match[2] || '0');
    if (!month || !Number.isInteger(day) || day < 1 || day > 31) {
      continue;
    }

    let year = Number(match[3] || String(localNow.year));
    if (!Number.isInteger(year) || year < 1000) {
      year = localNow.year;
    }
    if (!isValidCalendarDate(year, month, day)) {
      continue;
    }

    let iso = toIsoDate(year, month, day);
    if (!match[3] && iso < todayISO) {
      if (!isValidCalendarDate(year + 1, month, day)) {
        continue;
      }
      iso = toIsoDate(year + 1, month, day);
    }

    const start = match.index ?? -1;
    const end = start + match[0].length;
    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    ctx.result.fields.date = iso;
    const confidence = match[3] ? 0.9 : 0.84;
    addFieldSpan(ctx, 'date', {
      start,
      end,
      text: match[0],
      ruleId: 'datetime.month_day_fallback',
      confidence,
    });
    setFieldConfidence(ctx, 'date', confidence);
    addDebugStep(ctx, {
      pass: 'datetime',
      ruleId: 'datetime.month_day_fallback',
      start,
      end,
      text: match[0],
      confidence,
      note: 'date extracted from explicit month-day pattern',
    });
    maskMatch(ctx, {
      start,
      end,
      text: match[0],
      ruleId: 'datetime.month_day_fallback.mask',
      confidence,
    });
    return;
  }
};

const applyRecurrenceStartDateFallback = (ctx: ParseContext): void => {
  if (!ctx.result.fields.recurrence || ctx.result.fields.recurrence_start) {
    return;
  }

  const startMatch = RECURRENCE_START_REGEX.exec(ctx.rawInput);
  if (!startMatch) {
    return;
  }

  const snippet = startMatch[1] || '';
  if (!snippet.trim()) {
    return;
  }

  const parsed = chrono.parse(snippet, ctx.now, { forwardDate: true });
  if (parsed.length === 0) {
    return;
  }

  const bestDate = pickBestResult(parsed, dateScore);
  if (!bestDate) {
    return;
  }

  const dateISO = dateIsoFromComponent(bestDate.start, ctx.options.timezone);
  if (!dateISO) {
    return;
  }

  void findSubSpan(bestDate, DATE_FRAGMENT_PATTERNS);
  const confidence = 0.86;

  ctx.result.fields.recurrence_start = dateISO;
  addFieldSpan(ctx, 'recurrence_start', {
    start: startMatch.index,
    end: startMatch.index + startMatch[0].length,
    text: startMatch[0],
    ruleId: 'datetime.recurrence_start_date',
    confidence,
  });
  setFieldConfidence(ctx, 'recurrence_start', confidence);
  addDebugStep(ctx, {
    pass: 'datetime',
    ruleId: 'datetime.recurrence_start_date',
    start: startMatch.index,
    end: startMatch.index + startMatch[0].length,
    text: startMatch[0],
    confidence,
    note: 'recurrence start date inferred from starting phrase',
  });
  maskMatch(ctx, {
    start: startMatch.index,
    end: startMatch.index + startMatch[0].length,
    text: startMatch[0],
    ruleId: 'datetime.recurrence_start_date.mask',
    confidence,
  });
};

const applyUnsupportedRelativeTimeMasking = (ctx: ParseContext): void => {
  const matches = Array.from(ctx.maskedInput.matchAll(UNSUPPORTED_RELATIVE_TIME_RANGE_REGEX));

  for (const match of matches) {
    const fullText = match[0];
    const start = match.index ?? -1;
    const end = start + fullText.length;
    if (start < 0 || !isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const startClause = match[1] || '';
    if (EXPLICIT_CLOCK_SIGNAL_REGEX.test(fullText)) {
      continue;
    }
    if (!/\b(?:usual|regular|normal|sync|standup|meeting|call)\b/i.test(startClause) && !/\bpast the hour\b/i.test(startClause)) {
      continue;
    }

    addDebugStep(ctx, {
      pass: 'datetime',
      ruleId: 'datetime.unsupported_relative_anchor_time',
      start,
      end,
      text: fullText,
      confidence: 0.5,
      note: 'masked unsupported anchor-relative time clause to keep the lightweight parser deterministic',
    });
    maskMatch(ctx, {
      start,
      end,
      text: fullText,
      ruleId: 'datetime.unsupported_relative_anchor_time.mask',
      confidence: 0.5,
    });
  }
};

const applyUnsupportedSpeculativeTemporalMasking = (ctx: ParseContext): void => {
  const patterns = [
    UNSUPPORTED_APPROXIMATE_TIME_WINDOW_REGEX,
    UNSUPPORTED_SPECULATIVE_DURATION_REGEX,
    UNSUPPORTED_RESCHEDULE_CLAUSE_REGEX,
  ];

  for (const pattern of patterns) {
    const matches = Array.from(ctx.maskedInput.matchAll(pattern));

    for (const match of matches) {
      const fullText = match[0];
      const start = match.index ?? -1;
      const end = start + fullText.length;
      if (start < 0 || !isSpanAvailable(ctx, start, end)) {
        continue;
      }

      addDebugStep(ctx, {
        pass: 'datetime',
        ruleId: 'datetime.unsupported_speculative_temporal_clause',
        start,
        end,
        text: fullText,
        confidence: 0.48,
        note: 'masked unsupported speculative temporal clause to keep the lightweight parser deterministic',
      });
      maskMatch(ctx, {
        start,
        end,
        text: fullText,
        ruleId: 'datetime.unsupported_speculative_temporal_clause.mask',
        confidence: 0.48,
      });
    }
  }
};

const applyDateSpecialFallback = (
  ctx: ParseContext,
  regex: RegExp,
  dateISO: string,
  confidence: number,
  ruleId: string,
  note: string,
): void => {
  const match = regex.exec(ctx.rawInput);
  if (!match || match.index === undefined) {
    return;
  }

  const start = match.index;
  const end = start + match[0].length;
  const text = match[0];
  ctx.result.fields.date = dateISO;
  addFieldSpan(ctx, 'date', {
    start,
    end,
    text,
    ruleId,
    confidence,
  });
  setFieldConfidence(ctx, 'date', confidence);
  addDebugStep(ctx, {
    pass: 'datetime',
    ruleId,
    start,
    end,
    text,
    confidence,
    note,
  });
  maskMatch(ctx, {
    start,
    end,
    text,
    ruleId: `${ruleId}.mask`,
    confidence,
  });
};

const applyTimeSpecialFallback = (
  ctx: ParseContext,
  regex: RegExp,
  timeISO: string,
  confidence: number,
  ruleId: string,
  note: string,
): void => {
  const match = regex.exec(ctx.rawInput);
  if (!match || match.index === undefined) {
    return;
  }

  const start = match.index;
  const end = start + match[0].length;
  const text = match[0];
  applyTimeValue(ctx, {
    time: timeISO,
    endTime: null,
    durationMinutes: null,
    start,
    end,
    text,
    ruleId,
    confidence,
    note,
  });
};

const applySimpleDateFallbacks = (ctx: ParseContext): void => {
  const localNow = getZonedDateTimeParts(ctx.now, ctx.options.timezone);
  const todayISO = toIsoDate(localNow.year, localNow.month, localNow.day);
  const endOfDay = endOfDayReference(ctx.now, ctx.options.timezone);

  if (ctx.result.fields.date === null && /\b(?:by|before|at)?\s*(?:end of (?:the )?day|eod)\b/i.test(ctx.rawInput)) {
    applyDateSpecialFallback(
      ctx,
      /\b(?:by|before|at)?\s*(?:end of (?:the )?day|eod)\b/i,
      endOfDay.dateISO,
      0.82,
      'datetime.special.end_of_day_date',
      'date inferred from end-of-day expression',
    );
  }

  if (ctx.result.fields.date === null && /\b(?:by|before|at)?\s*(?:end of (?:the )?month)\b/i.test(ctx.rawInput)) {
    applyDateSpecialFallback(
      ctx,
      /\b(?:by|before|at)?\s*(?:end of (?:the )?month)\b/i,
      endOfMonthIso(ctx.now, ctx.options.timezone),
      0.84,
      'datetime.special.end_of_month',
      'date inferred from end-of-month expression',
    );
  }

  if (ctx.result.fields.date === null && /\b(?:by|before|at)?\s*(?:end of (?:the )?week|eow)\b/i.test(ctx.rawInput)) {
    applyDateSpecialFallback(
      ctx,
      /\b(?:by|before|at)?\s*(?:end of (?:the )?week|eow)\b/i,
      fridayOfCurrentWeekIso(ctx.now, ctx.options.timezone),
      0.83,
      'datetime.special.end_of_week',
      'date inferred from end-of-week expression',
    );
  }

  if (ctx.result.fields.date === null && /\btoday\b/i.test(ctx.rawInput)) {
    ctx.result.fields.date = todayISO;
    setFieldConfidence(ctx, 'date', 0.8);
  }

  if (ctx.result.fields.date === null && (/\btomorrow\b/i.test(ctx.rawInput) || /\btmr\b/i.test(ctx.rawInput))) {
    ctx.result.fields.date = addDaysToIsoDate(todayISO, 1);
    setFieldConfidence(ctx, 'date', 0.78);
  }
};

const applySimpleTimeFallbacks = (ctx: ParseContext): void => {
  const endOfDay = endOfDayReference(ctx.now, ctx.options.timezone);
  if (ctx.result.fields.time === null && /\b(?:end of day|eod)\b/i.test(ctx.rawInput)) {
    applyTimeSpecialFallback(
      ctx,
      /\b(?:by|before|at)?\s*(?:end of day|eod)\b/i,
      endOfDay.timeISO,
      0.8,
      'datetime.special.end_of_day_time',
      'time inferred from end-of-day expression',
    );
  }

  if (ctx.result.fields.time === null && /\b(?:end of (?:the )?week|eow)\b/i.test(ctx.rawInput)) {
    applyTimeSpecialFallback(
      ctx,
      /\b(?:by|before|at)?\s*(?:end of (?:the )?week|eow)\b/i,
      '17:00',
      0.78,
      'datetime.special.end_of_week_time',
      'time inferred from end-of-week expression',
    );
  }

  if (ctx.result.fields.time === null && /\bfirst\s+thing\b/i.test(ctx.rawInput)) {
    ctx.result.fields.time = '09:00';
    setFieldConfidence(ctx, 'time', 0.68);
  }
};

const applySpecialTemporalMasking = (ctx: ParseContext): void => {
  const patterns = [
    /\b(?:by|before|at)?\s*(?:end of (?:the )?day|eod)\b/gi,
    /\b(?:by|before|at)?\s*(?:end of (?:the )?week|eow)\b/gi,
    /\b(?:by|before|at)?\s*(?:end of (?:the )?month)\b/gi,
    /\b(?:today|tomorrow|tonight)\s+(?:morning|afternoon|evening|night)\b/gi,
  ];

  for (const pattern of patterns) {
    for (const match of ctx.rawInput.matchAll(pattern)) {
      const text = match[0] || '';
      const start = match.index ?? -1;
      if (start < 0 || !text) {
        continue;
      }
      const end = start + text.length;
      if (/\S/.test(ctx.maskedInput.slice(start, end))) {
        maskMatch(ctx, {
          start,
          end,
          text,
          ruleId: 'datetime.special.mask_cleanup',
          confidence: 0.7,
        });
      }
    }
  }
};

export const chronoPass = (ctx: ParseContext): void => {
  applyExplicitRangeFallback(ctx);
  applyMonthDayFallback(ctx);
  applyUnsupportedRelativeTimeMasking(ctx);
  applyUnsupportedSpeculativeTemporalMasking(ctx);

  const chronoInput = preprocessForChrono(ctx);
  const results = chrono.parse(chronoInput, ctx.now, { forwardDate: true });

  if (ctx.result.fields.date === null) {
    const bestDate = pickBestResult(results, dateScore);
    if (bestDate) {
      applyDateFromChrono(ctx, bestDate);
    }
  }

  if (ctx.result.fields.time === null) {
    const bestTime = pickBestResult(results, timeScore);
    if (bestTime) {
      applyTimeFromChrono(ctx, bestTime);
    }
  }

  applyRecurrenceStartDateFallback(ctx);
  applyRelativeWeekdayOverride(ctx);
  applyExplicitSingleTimeFallback(ctx);
  applyBareHourFallback(ctx);

  applySimpleDateFallbacks(ctx);
  applySimpleTimeFallbacks(ctx);
  applySpecialTemporalMasking(ctx);

  if (ctx.result.fields.date && /\b(?:this|next|week|mon|tue|wed|thu|fri|sat|sun)\b/i.test(ctx.rawInput)) {
    const nowParts = getZonedDateTimeParts(ctx.now, ctx.options.timezone);
    const todayISO = toIsoDate(nowParts.year, nowParts.month, nowParts.day);
    if (ctx.result.fields.date < todayISO) {
      ctx.result.fields.date = todayISO;
      addDebugStep(ctx, {
        pass: 'datetime',
        ruleId: 'datetime.guard.no_past_weekday',
        start: 0,
        end: ctx.rawInput.length,
        text: ctx.rawInput,
        confidence: 0.4,
        note: 'adjusted weekday date to avoid past resolution',
      });
      setFieldConfidence(ctx, 'date', 0.4);
    }
  }
};
