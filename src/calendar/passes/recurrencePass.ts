import * as chrono from 'chrono-node';
import { WEEKDAY_LIST_PATTERN } from '../constants.ts';
import type { ParseContext, RecurrenceFrequency, RecurrenceSignal } from '../types.ts';
import { classifyRecurrenceException } from '../recurrenceExceptions.ts';
import {
  addDebugStep,
  addFieldSpan,
  formatDateInTimezone,
  getZonedDateTimeParts,
  isSpanAvailable,
  maskMatch,
  normalizeWeekdayList,
  parseNumberToken,
  setFieldConfidence,
  toIsoDate,
} from '../utils.ts';

interface RecurrenceRule {
  id: string;
  regex: RegExp;
  confidence: number;
  frequency: RecurrenceFrequency;
  interval?: number;
  intervalGroup?: number;
  daysGroup?: number;
  days?: string[];
  ordinal?: number;
  ordinalGroup?: number;
  note: string;
}

const DAY = WEEKDAY_LIST_PATTERN;
const DAY_LIST = `${DAY}(?:\\s*(?:,|and)\\s*${DAY})*`;
const COUNT = '(\\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)';
const ORDINAL = '(first|1st|second|2nd|third|3rd|fourth|4th|last)';
const RECURRENCE_SIGNAL_REGEX =
  /\b(?:every|daily|weekly|monthly|yearly|annually|bi-?weekly|fortnight|weekdays?|weekends?|every\s+other)\b/i;
const RECURRENCE_DATE_SIGNAL_REGEX =
  /\b(?:today|tomorrow|next|this|week|month|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?|jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t|tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?|\d{4}-\d{2}-\d{2}|\d{1,2}[/-]\d{1,2})\b/i;
const WEEKDAY_TO_RRULE: Record<string, string> = {
  monday: 'MO',
  tuesday: 'TU',
  wednesday: 'WE',
  thursday: 'TH',
  friday: 'FR',
  saturday: 'SA',
  sunday: 'SU',
};

const RECURRENCE_RULES: RecurrenceRule[] = [
  {
    id: 'recurrence.every_n_weeks_on_days',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+weeks?\\s+on\\s+(${DAY_LIST})\\b`, 'gi'),
    confidence: 0.93,
    frequency: 'weekly',
    intervalGroup: 1,
    daysGroup: 2,
    note: 'numeric weekly recurrence with explicit days',
  },
  {
    id: 'recurrence.every_n_months_on_nth_weekday',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+months?\\s+on\\s+the\\s+${ORDINAL}\\s+(${DAY})\\b`, 'gi'),
    confidence: 0.9,
    frequency: 'monthly',
    intervalGroup: 1,
    ordinalGroup: 2,
    daysGroup: 3,
    note: 'numeric monthly recurrence with ordinal weekday',
  },
  {
    id: 'recurrence.every_month_on_nth_weekday',
    regex: new RegExp(`\\bevery\\s+month\\s+on\\s+the\\s+${ORDINAL}\\s+(${DAY})\\b`, 'gi'),
    confidence: 0.89,
    frequency: 'monthly',
    interval: 1,
    ordinalGroup: 1,
    daysGroup: 2,
    note: 'monthly recurrence with ordinal weekday',
  },
  {
    id: 'recurrence.every_nth_weekday_of_month',
    regex: new RegExp(`\\bevery\\s+${ORDINAL}\\s+(${DAY})\\s+of\\s+the\\s+month\\b`, 'gi'),
    confidence: 0.92,
    frequency: 'monthly',
    interval: 1,
    ordinalGroup: 1,
    daysGroup: 2,
    note: 'monthly recurrence phrased as every second Tuesday of the month',
  },
  {
    id: 'recurrence.every_other_weekday',
    regex: new RegExp(`\\bevery\\s+other\\s+(${DAY})\\b`, 'gi'),
    confidence: 0.9,
    frequency: 'weekly',
    interval: 2,
    daysGroup: 1,
    note: 'every other specific weekday',
  },
  {
    id: 'recurrence.every_week_on_days',
    regex: new RegExp(`\\bevery\\s+week\\s+on\\s+(${DAY_LIST})\\b`, 'gi'),
    confidence: 0.94,
    frequency: 'weekly',
    interval: 1,
    daysGroup: 1,
    note: 'weekly recurrence with day list',
  },
  {
    id: 'recurrence.weekly_on_days',
    regex: new RegExp(`\\bweekly\\s+on\\s+(${DAY_LIST})\\b`, 'gi'),
    confidence: 0.94,
    frequency: 'weekly',
    interval: 1,
    daysGroup: 1,
    note: 'weekly on <days>',
  },
  {
    id: 'recurrence.on_days_weekly',
    regex: new RegExp(`\\bon\\s+(${DAY_LIST})\\s+weekly\\b`, 'gi'),
    confidence: 0.9,
    frequency: 'weekly',
    interval: 1,
    daysGroup: 1,
    note: 'on <days> weekly',
  },
  {
    id: 'recurrence.every_weekday',
    regex: /\bevery\s+weekday\b/gi,
    confidence: 0.93,
    frequency: 'weekly',
    interval: 1,
    days: ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'],
    note: 'weekday recurrence phrase',
  },
  {
    id: 'recurrence.weekdays',
    regex: /\bweekdays\b/gi,
    confidence: 0.83,
    frequency: 'weekly',
    interval: 1,
    days: ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'],
    note: 'weekdays shorthand',
  },
  {
    id: 'recurrence.every_weekend',
    regex: /\bevery\s+weekend\b/gi,
    confidence: 0.83,
    frequency: 'weekly',
    interval: 1,
    days: ['saturday', 'sunday'],
    note: 'weekend recurrence phrase',
  },
  {
    id: 'recurrence.weekends',
    regex: /\bweekends\b/gi,
    confidence: 0.75,
    frequency: 'weekly',
    interval: 1,
    days: ['saturday', 'sunday'],
    note: 'weekends shorthand',
  },
  {
    id: 'recurrence.every_other_day',
    regex: /\bevery\s+other\s+day\b/gi,
    confidence: 0.92,
    frequency: 'daily',
    interval: 2,
    note: 'every other day recurrence',
  },
  {
    id: 'recurrence.every_single_day',
    regex: /\bevery\s+single\s+day\b/gi,
    confidence: 0.88,
    frequency: 'daily',
    interval: 1,
    note: 'explicit every single day phrase',
  },
  {
    id: 'recurrence.every_day',
    regex: /\bevery\s+day\b/gi,
    confidence: 0.95,
    frequency: 'daily',
    interval: 1,
    note: 'every day phrase',
  },
  {
    id: 'recurrence.each_day',
    regex: /\beach\s+day\b/gi,
    confidence: 0.86,
    frequency: 'daily',
    interval: 1,
    note: 'each day phrase',
  },
  {
    id: 'recurrence.daily',
    regex: /\bdaily\b/gi,
    confidence: 0.95,
    frequency: 'daily',
    interval: 1,
    note: 'daily keyword',
  },
  {
    id: 'recurrence.every_other_week',
    regex: /\bevery\s+other\s+week\b/gi,
    confidence: 0.9,
    frequency: 'weekly',
    interval: 2,
    note: 'every other week phrase',
  },
  {
    id: 'recurrence.biweekly',
    regex: /\bbi-?weekly\b/gi,
    confidence: 0.9,
    frequency: 'weekly',
    interval: 2,
    note: 'biweekly keyword',
  },
  {
    id: 'recurrence.fortnight',
    regex: /\bevery\s+fortnight\b/gi,
    confidence: 0.87,
    frequency: 'weekly',
    interval: 2,
    note: 'fortnight recurrence',
  },
  {
    id: 'recurrence.every_week',
    regex: /\bevery\s+week\b/gi,
    confidence: 0.91,
    frequency: 'weekly',
    interval: 1,
    note: 'every week phrase',
  },
  {
    id: 'recurrence.each_week',
    regex: /\beach\s+week\b/gi,
    confidence: 0.83,
    frequency: 'weekly',
    interval: 1,
    note: 'each week phrase',
  },
  {
    id: 'recurrence.weekly',
    regex: /\bweekly\b/gi,
    confidence: 0.9,
    frequency: 'weekly',
    interval: 1,
    note: 'weekly keyword',
  },
  {
    id: 'recurrence.every_n_days',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+days?\\b`, 'gi'),
    confidence: 0.92,
    frequency: 'daily',
    intervalGroup: 1,
    note: 'numeric day interval',
  },
  {
    id: 'recurrence.every_n_months_on_days',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+months?\\s+on\\s+(${DAY_LIST})\\b`, 'gi'),
    confidence: 0.9,
    frequency: 'monthly',
    intervalGroup: 1,
    daysGroup: 2,
    note: 'numeric monthly recurrence with explicit day list',
  },
  {
    id: 'recurrence.every_n_weeks',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+weeks?\\b`, 'gi'),
    confidence: 0.91,
    frequency: 'weekly',
    intervalGroup: 1,
    note: 'numeric week interval',
  },
  {
    id: 'recurrence.every_n_months',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+months?\\b`, 'gi'),
    confidence: 0.91,
    frequency: 'monthly',
    intervalGroup: 1,
    note: 'numeric month interval',
  },
  {
    id: 'recurrence.every_n_years',
    regex: new RegExp(`\\bevery\\s+${COUNT}\\s+years?\\b`, 'gi'),
    confidence: 0.91,
    frequency: 'yearly',
    intervalGroup: 1,
    note: 'numeric year interval',
  },
  {
    id: 'recurrence.every_weekday_name',
    regex: new RegExp(`\\bevery\\s+(${DAY})\\b`, 'gi'),
    confidence: 0.88,
    frequency: 'weekly',
    interval: 1,
    daysGroup: 1,
    note: 'every <weekday> recurrence',
  },
  {
    id: 'recurrence.twice_a_week',
    regex: /\btwice\s+a\s+week\b/gi,
    confidence: 0.62,
    frequency: 'weekly',
    interval: 1,
    note: 'ambiguous twice a week fallback',
  },
  {
    id: 'recurrence.twice_weekly',
    regex: /\btwice\s+weekly\b/gi,
    confidence: 0.62,
    frequency: 'weekly',
    interval: 1,
    note: 'ambiguous twice weekly fallback',
  },
  {
    id: 'recurrence.monthly_on_day_of_month',
    regex: /\b(?:every\s+month|monthly)\s+on\s+the\s+\d{1,2}(?:st|nd|rd|th)?\b/gi,
    confidence: 0.82,
    frequency: 'monthly',
    interval: 1,
    note: 'monthly recurrence anchored to day-of-month phrase',
  },
  {
    id: 'recurrence.every_month',
    regex: /\bevery\s+month\b/gi,
    confidence: 0.93,
    frequency: 'monthly',
    interval: 1,
    note: 'every month phrase',
  },
  {
    id: 'recurrence.each_month',
    regex: /\beach\s+month\b/gi,
    confidence: 0.82,
    frequency: 'monthly',
    interval: 1,
    note: 'each month phrase',
  },
  {
    id: 'recurrence.monthly',
    regex: /\bmonthly\b/gi,
    confidence: 0.93,
    frequency: 'monthly',
    interval: 1,
    note: 'monthly keyword',
  },
  {
    id: 'recurrence.every_year',
    regex: /\bevery\s+year\b/gi,
    confidence: 0.93,
    frequency: 'yearly',
    interval: 1,
    note: 'every year phrase',
  },
  {
    id: 'recurrence.yearly',
    regex: /\byearly\b/gi,
    confidence: 0.93,
    frequency: 'yearly',
    interval: 1,
    note: 'yearly keyword',
  },
  {
    id: 'recurrence.annually',
    regex: /\bannually\b/gi,
    confidence: 0.9,
    frequency: 'yearly',
    interval: 1,
    note: 'annually keyword',
  },
];

const ordinalFromToken = (token: string | null | undefined): number | null => {
  const normalized = (token || '').trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized === 'first' || normalized === '1st') return 1;
  if (normalized === 'second' || normalized === '2nd') return 2;
  if (normalized === 'third' || normalized === '3rd') return 3;
  if (normalized === 'fourth' || normalized === '4th') return 4;
  if (normalized === 'last') return -1;
  return null;
};

const nextMonthStartIso = (ctx: ParseContext): string => {
  const parts = getZonedDateTimeParts(ctx.now, ctx.options.timezone);
  const month = parts.month === 12 ? 1 : parts.month + 1;
  const year = parts.month === 12 ? parts.year + 1 : parts.year;
  return toIsoDate(year, month, 1);
};

const nextWeekStartIso = (ctx: ParseContext): string => {
  const parts = getZonedDateTimeParts(ctx.now, ctx.options.timezone);
  const currentDate = new Date(Date.UTC(parts.year, parts.month - 1, parts.day));
  const currentIsoWeekday = (currentDate.getUTCDay() + 6) % 7;
  const delta = 7 - currentIsoWeekday;
  const target = new Date(currentDate.getTime() + delta * 86400000);
  return toIsoDate(target.getUTCFullYear(), target.getUTCMonth() + 1, target.getUTCDate());
};

const buildRecurrenceRule = (signal: RecurrenceSignal): string | null => {
  if (!signal.frequency || !signal.interval || signal.interval <= 0) {
    return null;
  }

  const freq = signal.frequency.toUpperCase();
  let rule = `FREQ=${freq};INTERVAL=${signal.interval}`;
  if (signal.days && signal.days.length > 0) {
    const tokens = signal.days.map((day) => WEEKDAY_TO_RRULE[day]).filter(Boolean);
    if (tokens.length > 0) {
      if (signal.frequency === 'monthly' && signal.ordinal && tokens.length === 1) {
        rule += `;BYDAY=${signal.ordinal}${tokens[0]}`;
      } else {
        rule += `;BYDAY=${tokens.join(',')}`;
      }
    }
  }
  return rule;
};

const extractIsoDateFromSnippet = (ctx: ParseContext, snippet: string): string | null => {
  const cleaned = snippet.trim();
  const lowered = cleaned.toLowerCase();
  if (!cleaned) {
    return null;
  }

  if (lowered === 'next month') {
    return nextMonthStartIso(ctx);
  }

  if (lowered === 'next week') {
    return nextWeekStartIso(ctx);
  }

  const parsed = chrono.parse(cleaned, ctx.now, { forwardDate: true })[0];
  if (!parsed) {
    return null;
  }

  const year = parsed.start.get('year');
  const month = parsed.start.get('month');
  const day = parsed.start.get('day');

  if (typeof year === 'number' && typeof month === 'number' && typeof day === 'number') {
    return toIsoDate(year, month, day);
  }

  return formatDateInTimezone(parsed.start.date(), ctx.options.timezone);
};

const pushException = (ctx: ParseContext, raw: string): void => {
  const current = ctx.result.fields.recurrence_exception;
  if (!current.some((exception) => exception.raw === raw)) {
    current.push(
      classifyRecurrenceException(raw, {
        now: ctx.now,
        timezone: ctx.options.timezone,
      }),
    );
  }
};

const applyRecurrence = (
  ctx: ParseContext,
  payload: RecurrenceSignal,
  start: number,
  end: number,
  text: string,
  ruleId: string,
  confidence: number,
  note: string,
): void => {
  const recurrenceRule = buildRecurrenceRule(payload);
  if (!recurrenceRule) {
    return;
  }

  let resolvedConfidence = confidence;
  if (ctx.signals.recurrence.frequency && ctx.signals.recurrence.frequency !== payload.frequency) {
    resolvedConfidence = Math.max(0.35, confidence - 0.25);
  }

  if (ctx.result.fields.recurrence && resolvedConfidence < ctx.result.meta.confidence.recurrence) {
    return;
  }

  ctx.signals.recurrence = {
    frequency: payload.frequency,
    interval: payload.interval,
    days: payload.days,
    ordinal: payload.ordinal,
  };
  ctx.result.fields.recurrence = recurrenceRule;

  addFieldSpan(ctx, 'recurrence', {
    start,
    end,
    text,
    ruleId,
    confidence: resolvedConfidence,
  });
  setFieldConfidence(ctx, 'recurrence', resolvedConfidence);
  addDebugStep(ctx, {
    pass: 'recurrence',
    ruleId,
    start,
    end,
    text,
    confidence: resolvedConfidence,
    note,
  });
  maskMatch(ctx, { start, end, text, ruleId, confidence: resolvedConfidence });
};

const runStartRule = (ctx: ParseContext): void => {
  if (!ctx.result.fields.recurrence && !RECURRENCE_SIGNAL_REGEX.test(ctx.rawInput)) {
    return;
  }

  const regex =
    /\b(?:starting|beginning)\s+(.+?)(?=(?:\s+\b(?:unless|except|but|skip|remind|alert|with|invite|w\/|at|for|until|till|through|ending)\b|[,.;]|$))/gi;
  const matches = Array.from(ctx.maskedInput.matchAll(regex));

  for (const match of matches) {
    const fullText = match[0];
    const start = match.index ?? -1;
    const end = start + fullText.length;
    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const dateText = match[1] || '';
    const isoDate = extractIsoDateFromSnippet(ctx, dateText);
    if (!isoDate) {
      continue;
    }

    ctx.result.fields.recurrence_start = isoDate;
    addFieldSpan(ctx, 'recurrence_start', {
      start,
      end,
      text: fullText,
      ruleId: 'recurrence.start',
      confidence: 0.88,
    });
    setFieldConfidence(ctx, 'recurrence_start', 0.88);
    addDebugStep(ctx, {
      pass: 'recurrence',
      ruleId: 'recurrence.start',
      start,
      end,
      text: fullText,
      confidence: 0.88,
      note: 'detected recurrence start boundary',
    });
    maskMatch(ctx, { start, end, text: fullText, ruleId: 'recurrence.start', confidence: 0.88 });
  }
};

const runEndDateRule = (ctx: ParseContext): void => {
  if (!ctx.result.fields.recurrence && !RECURRENCE_SIGNAL_REGEX.test(ctx.rawInput)) {
    return;
  }

  const regex =
    /\b(?:until|till|through|ending)\s+(.+?)(?=(?:\s+\b(?:unless|except|but|skip|remind|alert|with|invite|w\/|every|for|at|on|from|to|starting|beginning)\b|[,.;]|$))/gi;
  const matches = Array.from(ctx.maskedInput.matchAll(regex));

  for (const match of matches) {
    const fullText = match[0];
    const start = match.index ?? -1;
    const end = start + fullText.length;
    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const dateText = match[1] || '';
    if (!RECURRENCE_DATE_SIGNAL_REGEX.test(dateText)) {
      continue;
    }

    const isoDate = extractIsoDateFromSnippet(ctx, dateText);
    if (!isoDate) {
      continue;
    }

    ctx.result.fields.recurrence_end = isoDate;
    addFieldSpan(ctx, 'recurrence_end', {
      start,
      end,
      text: fullText,
      ruleId: 'recurrence.end',
      confidence: 0.84,
    });
    setFieldConfidence(ctx, 'recurrence_end', 0.84);
    addDebugStep(ctx, {
      pass: 'recurrence',
      ruleId: 'recurrence.end',
      start,
      end,
      text: fullText,
      confidence: 0.84,
      note: 'detected recurrence end boundary from until/till/ending phrase',
    });
    maskMatch(ctx, { start, end, text: fullText, ruleId: 'recurrence.end', confidence: 0.84 });
  }
};

const runExceptionRule = (ctx: ParseContext): void => {
  if (!ctx.result.fields.recurrence && !RECURRENCE_SIGNAL_REGEX.test(ctx.rawInput)) {
    return;
  }

  const rules: Array<{ id: string; regex: RegExp; note: string }> = [
    {
      id: 'recurrence.exception.unless',
      regex:
        /\bunless\s+(.+?)(?=(?:\s+\b(?:starting|beginning|but|at|for|until|till|through|ending|remind|alert)\b|[,.;]|$))/gi,
      note: 'captured recurrence exception from unless clause',
    },
    {
      id: 'recurrence.exception.except',
      regex:
        /\bexcept(?:\s+for)?\s+(.+?)(?=(?:\s+\b(?:starting|beginning|but|at|for|until|till|through|ending|remind|alert)\b|[,.;]|$))/gi,
      note: 'captured recurrence exception from except clause',
    },
    {
      id: 'recurrence.exception.but_skip',
      regex:
        /\bbut\s+skip\s+(.+?)(?=(?:\s+\b(?:starting|beginning|at|for|until|till|through|ending|remind|alert)\b|[,.;]|$))/gi,
      note: 'captured recurrence exception from but skip clause',
    },
    {
      id: 'recurrence.exception.but_not',
      regex:
        /\bbut\s+not\s+(.+?)(?=(?:\s+\b(?:starting|beginning|at|for|until|till|through|ending|remind|alert)\b|[,.;]|$))/gi,
      note: 'captured recurrence exception from but not clause',
    },
    {
      id: 'recurrence.exception.skip_first',
      regex: /\bskip\s+the\s+first\s+one\b/gi,
      note: 'captured explicit skip-the-first recurrence exception',
    },
  ];

  for (const rule of rules) {
    const matches = Array.from(ctx.maskedInput.matchAll(rule.regex));

    for (const match of matches) {
      const fullText = match[0].trim().replace(/[.,;]+$/g, '');
      const start = match.index ?? -1;
      const end = start + fullText.length;
      if (!fullText || !isSpanAvailable(ctx, start, end)) {
        continue;
      }

      pushException(ctx, fullText);
      addFieldSpan(ctx, 'recurrence_exception', {
        start,
        end,
        text: fullText,
        ruleId: rule.id,
        confidence: 0.8,
      });
      setFieldConfidence(ctx, 'recurrence_exception', 0.8);
      addDebugStep(ctx, {
        pass: 'recurrence',
        ruleId: rule.id,
        start,
        end,
        text: fullText,
        confidence: 0.8,
        note: rule.note,
      });
      maskMatch(ctx, { start, end, text: fullText, ruleId: rule.id, confidence: 0.8 });
    }
  }
};

const runRecurrenceRules = (ctx: ParseContext): void => {
  for (const rule of RECURRENCE_RULES) {
    const matches = Array.from(ctx.maskedInput.matchAll(rule.regex));

    for (const match of matches) {
      const fullText = match[0];
      const start = match.index ?? -1;
      const end = start + fullText.length;

      if (!isSpanAvailable(ctx, start, end)) {
        continue;
      }

      const intervalToken =
        typeof rule.intervalGroup === 'number' && typeof match[rule.intervalGroup] === 'string'
          ? match[rule.intervalGroup]
          : null;
      const intervalFromToken = intervalToken ? parseNumberToken(intervalToken) : null;
      const interval = rule.interval ?? (intervalFromToken && intervalFromToken > 0 ? Math.round(intervalFromToken) : null);

      if (!interval || interval <= 0) {
        continue;
      }

      const daysFromGroup =
        typeof rule.daysGroup === 'number' && typeof match[rule.daysGroup] === 'string'
          ? normalizeWeekdayList(match[rule.daysGroup])
          : null;
      const ordinalToken =
        typeof rule.ordinalGroup === 'number' && typeof match[rule.ordinalGroup] === 'string'
          ? match[rule.ordinalGroup]
          : null;
      const ordinal = rule.ordinal ?? ordinalFromToken(ordinalToken);
      const resolvedDays = rule.days || daysFromGroup || null;

      applyRecurrence(
        ctx,
        {
          frequency: rule.frequency,
          interval,
          days: resolvedDays,
          ordinal: ordinal ?? null,
        },
        start,
        end,
        fullText,
        rule.id,
        rule.confidence,
        rule.note,
      );
    }
  }
};

const runWeeklyInlineOnDaysRule = (ctx: ParseContext): void => {
  if (!/\bweekly\b/i.test(ctx.rawInput)) {
    return;
  }

  const regex = new RegExp(`\\bon\\s+(${DAY_LIST})\\b`, 'gi');
  const matches = Array.from(ctx.maskedInput.matchAll(regex));

  for (const match of matches) {
    const fullText = match[0];
    const start = match.index ?? -1;
    const end = start + fullText.length;
    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const days = normalizeWeekdayList(match[1] || '');
    if (!days || days.length === 0) {
      continue;
    }

    applyRecurrence(
      ctx,
      {
        frequency: 'weekly',
        interval: 1,
        days,
        ordinal: null,
      },
      start,
      end,
      fullText,
      'recurrence.weekly_inline_on_days',
      0.9,
      'weekly phrase with explicit on <days> clause',
    );
  }
};

export const recurrencePass = (ctx: ParseContext): void => {
  runRecurrenceRules(ctx);
  runExceptionRule(ctx);
  runStartRule(ctx);
  runEndDateRule(ctx);
  runWeeklyInlineOnDaysRule(ctx);
};
