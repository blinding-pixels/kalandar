import type { EventParseResult, ParseContext, ParseWarning } from './types.ts';
import { normalizeWeekdayToken, weekdayOfISODate } from './utils.ts';

const WEEKDAY_TOKEN =
  '(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';
const WEEKDAY_REGEX = new RegExp(`\\b${WEEKDAY_TOKEN}\\b`, 'gi');
const EXPLICIT_DATE_REGEXES = [
  /\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t|tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}(?:st|nd|rd|th)?(?:,?\s+\d{4})?\b/gi,
  /\b\d{4}-\d{2}-\d{2}\b/g,
  /\b\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?\b/g,
];

const addMinutesToIsoTime = (time: string, minutesToAdd: number): string | null => {
  const match = /^(\d{2}):(\d{2})$/.exec(time);
  if (!match) {
    return null;
  }
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (!Number.isInteger(hour) || !Number.isInteger(minute)) {
    return null;
  }

  const total = hour * 60 + minute + minutesToAdd;
  const wrapped = ((total % (24 * 60)) + 24 * 60) % (24 * 60);
  const nextHour = Math.floor(wrapped / 60);
  const nextMinute = wrapped % 60;
  return `${String(nextHour).padStart(2, '0')}:${String(nextMinute).padStart(2, '0')}`;
};

const findWeekdaySpans = (input: string): Array<{ start: number; end: number; text: string; weekday: string }> => {
  const spans: Array<{ start: number; end: number; text: string; weekday: string }> = [];
  WEEKDAY_REGEX.lastIndex = 0;
  for (const match of input.matchAll(WEEKDAY_REGEX)) {
    const text = match[0] || '';
    const start = match.index ?? -1;
    const end = start + text.length;
    const weekday = normalizeWeekdayToken(text);
    if (start < 0 || !weekday) {
      continue;
    }
    spans.push({ start, end, text, weekday });
  }
  return spans;
};

const findExplicitDateTokenSpans = (input: string): Array<{ start: number; end: number; text: string }> => {
  const spans: Array<{ start: number; end: number; text: string }> = [];

  for (const regex of EXPLICIT_DATE_REGEXES) {
    regex.lastIndex = 0;
    for (const match of input.matchAll(regex)) {
      const text = match[0] || '';
      const start = match.index ?? -1;
      if (start < 0 || !text) {
        continue;
      }
      spans.push({ start, end: start + text.length, text });
    }
  }

  return spans;
};

const findRecurrenceWeekdayHintSpan = (
  rawInput: string,
  recurrenceSpans: EventParseResult['meta']['spans']['recurrence'],
): { start: number; end: number; text: string } | null => {
  for (const span of recurrenceSpans) {
    WEEKDAY_REGEX.lastIndex = 0;
    if (!WEEKDAY_REGEX.test(span.text)) {
      continue;
    }
    return { start: span.start, end: span.end, text: span.text };
  }

  const weekdaySpan = findWeekdaySpans(rawInput)[0];
  if (!weekdaySpan) {
    return null;
  }
  return { start: weekdaySpan.start, end: weekdaySpan.end, text: weekdaySpan.text };
};

export const finalizeCalendarParseContext = (ctx: ParseContext): EventParseResult => {
  if (ctx.result.fields.time && ctx.result.fields.end_time === null && ctx.result.fields.duration_minutes === null) {
    const endTime = addMinutesToIsoTime(ctx.result.fields.time, 60);
    if (endTime) {
      ctx.result.fields.end_time = endTime;
      ctx.result.fields.duration_minutes = 60;
      ctx.result.meta.confidence.end_time = Math.max(ctx.result.meta.confidence.end_time, 0.68);
      ctx.result.meta.confidence.duration_minutes = Math.max(ctx.result.meta.confidence.duration_minutes, 0.66);
      ctx.result.meta.debugSteps.push({
        pass: 'postprocess',
        ruleId: 'postprocess.default_end_time',
        start: 0,
        end: ctx.rawInput.length,
        text: ctx.result.fields.time,
        confidence: 0.68,
        note: 'defaulted end time to +60 minutes because no explicit end time was provided',
      });
    }
  }

  const warnings: ParseWarning[] = [...(ctx.result.warnings || [])];
  const recurrenceActive = Boolean(ctx.result.fields.recurrence);
  const recurrenceStart = ctx.result.fields.recurrence_start;
  const recurrenceStartSpan = ctx.result.meta.spans.recurrence_start[0] || null;

  const dateSpanOverlapsAnchor =
    recurrenceStartSpan !== null &&
    ctx.result.meta.spans.date.some(
      (span) => span.start < recurrenceStartSpan.end && span.end > recurrenceStartSpan.start,
    );

  if (recurrenceActive && recurrenceStart && (ctx.result.fields.date === recurrenceStart || dateSpanOverlapsAnchor)) {
    ctx.result.fields.date = null;
    ctx.result.meta.confidence.date = 0;
    ctx.result.meta.debugSteps.push({
      pass: 'postprocess',
      ruleId: 'postprocess.recurrence_anchor_date_suppressed',
      start: recurrenceStartSpan?.start ?? 0,
      end: recurrenceStartSpan?.end ?? 0,
      text: recurrenceStartSpan?.text ?? recurrenceStart,
      confidence: 0.82,
      note: 'suppressed one-off date because recurrence_start acts as series anchor',
    });
  }

  const coarseRecurrenceStart =
    recurrenceStartSpan !== null && /\bnext\s+(?:month|week)\b/i.test(recurrenceStartSpan.text);

  if (
    recurrenceActive &&
    recurrenceStart &&
    !coarseRecurrenceStart &&
    ctx.signals.recurrence.days &&
    ctx.signals.recurrence.days.length > 0
  ) {
    const anchorWeekday = weekdayOfISODate(recurrenceStart, ctx.options.timezone);
    if (anchorWeekday && !ctx.signals.recurrence.days.includes(anchorWeekday)) {
      const recurrenceSpan = findRecurrenceWeekdayHintSpan(ctx.rawInput, ctx.result.meta.spans.recurrence);
      const spanPayload = recurrenceSpan
        ? [
            {
              start: recurrenceStartSpan?.start ?? 0,
              end: recurrenceStartSpan?.end ?? 0,
              text: recurrenceStartSpan?.text ?? recurrenceStart,
            },
            recurrenceSpan,
          ]
        : [
            {
              start: recurrenceStartSpan?.start ?? 0,
              end: recurrenceStartSpan?.end ?? 0,
              text: recurrenceStartSpan?.text ?? recurrenceStart,
            },
          ];

      warnings.push({
        code: 'recurrence_start_weekday_mismatch',
        severity: 'warning',
        message: "Start date doesn't fall on the recurrence weekday; the series will begin on the next matching day.",
        fieldHints: ['recurrence', 'recurrence_start'],
        spans: spanPayload,
        details: {
          anchorDateISO: recurrenceStart,
          anchorWeekday,
          recurrenceDays: ctx.signals.recurrence.days,
          policy: 'snap_forward_no_backwards',
        },
      });
    }
  }

  if (!recurrenceActive && ctx.result.fields.date) {
    const explicitDateSpans = findExplicitDateTokenSpans(ctx.rawInput);
    if (explicitDateSpans.length > 0) {
      const weekdaySpans = findWeekdaySpans(ctx.rawInput);
      const computedWeekday = weekdayOfISODate(ctx.result.fields.date, ctx.options.timezone);
      if (weekdaySpans.length > 0 && computedWeekday) {
        const mismatches = weekdaySpans.filter((entry) => entry.weekday !== computedWeekday);
        if (mismatches.length > 0) {
          warnings.push({
            code: 'weekday_date_conflict',
            severity: 'warning',
            message: "Weekday doesn't match the given date.",
            fieldHints: ['date'],
            spans: mismatches.map((entry) => ({
              start: entry.start,
              end: entry.end,
              text: entry.text,
            })),
            details: {
              dateISO: ctx.result.fields.date,
              weekdayText: mismatches[0].text,
              computedWeekday,
              policy: 'date_wins',
            },
          });
        }
      }
    }
  }

  ctx.result.meta.maskedInput = ctx.maskedInput;
  ctx.result.warnings = warnings.length > 0 ? warnings : null;
  return ctx.result;
};
