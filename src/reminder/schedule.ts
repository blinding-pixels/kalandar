import { classifyRecurrenceException } from '../calendar/recurrenceExceptions.ts';
import type { CaptureSchedule, PreviewRecurrence, ScheduleRole, ScheduleSpan, ScheduleTexts } from '../dates/index.ts';
import type { ParseWarning } from '../shared/types.ts';
import type { ReminderFields, ReminderParseContext, ReminderRecurrence } from './types.ts';
import { addDebugStep, addFieldSpan, setFieldConfidence } from './utils.ts';

type ScheduleField =
  | 'remind_at'
  | 'recurrence'
  | 'recurrence_start'
  | 'recurrence_end'
  | 'recurrence_exception'
  | 'recurrence_window'
  | 'context_hint';

const CONTEXT_HINT_REGEX = /\b(end of (?:the )?(?:month|week|day)|next month|morning|noon|afternoon|evening|night)\b/i;

const ROLE_FIELDS: Partial<Record<ScheduleRole, ScheduleField>> = {
  trigger: 'remind_at',
  event_start: 'remind_at',
  due_date: 'remind_at',
  due_time: 'remind_at',
  recurrence: 'recurrence',
  recurrence_start: 'recurrence_start',
  recurrence_end: 'recurrence_end',
  recurrence_exception: 'recurrence_exception',
  recurrence_window: 'recurrence_window',
};

const FIELD_CONFIDENCE: Record<ScheduleField, number> = {
  remind_at: 0.88,
  recurrence: 0.9,
  recurrence_start: 0.96,
  recurrence_end: 0.96,
  recurrence_exception: 0.96,
  recurrence_window: 0.96,
  context_hint: 0.72,
};

const SCHEDULE_FIELDS = Object.keys(FIELD_CONFIDENCE) as ScheduleField[];

const hasValue = (value: ReminderFields[ScheduleField]): boolean =>
  Array.isArray(value) ? value.length > 0 : value !== null;

export const reminderClockTime = (texts: ScheduleTexts): string | null => {
  const time = texts.time;
  if (!time) {
    return null;
  }
  return /\d/.test(time) || /^(?:noon|midnight)$/i.test(time) ? time : null;
};

export const reminderRecurrence = (recurrence: PreviewRecurrence | null | undefined): ReminderRecurrence | null => {
  if (!recurrence) {
    return null;
  }
  const weekdays = recurrence.weekdays.filter((day) => day.position === null).map((day) => day.weekday);
  return {
    frequency: recurrence.frequency,
    interval: recurrence.interval,
    days: recurrence.days ?? (weekdays.length > 0 ? weekdays : null),
  };
};

export const reminderContextHint = (texts: ScheduleTexts): string | null => {
  for (const text of [texts.moment, texts.time, texts.recurrence]) {
    const match = text?.match(CONTEXT_HINT_REGEX);
    if (match) {
      return match[0].toLowerCase().replace('end of the ', 'end of ');
    }
  }
  return null;
};

const scheduleWarnings = (found: CaptureSchedule): ParseWarning[] =>
  found.warnings.map((warning) => ({
    code: warning.code,
    severity: 'info',
    message: warning.message,
    fieldHints: [],
    spans: [],
    details: {},
  }));

export const applyReminderSchedule = (
  ctx: ReminderParseContext,
  found: CaptureSchedule,
  pass: string,
  spanConfidence?: (span: ScheduleSpan) => number | null,
): void => {
  const { texts, schedule } = found;
  const fields = ctx.result.fields;
  fields.remind_at = texts.moment ?? reminderClockTime(texts);
  fields.recurrence = reminderRecurrence(schedule?.recurrence);
  fields.recurrence_start = texts.recurrence_start;
  fields.recurrence_end = texts.recurrence_end;
  fields.recurrence_window = texts.recurrence_window;
  fields.recurrence_exception = texts.recurrence_exceptions.map((raw) =>
    classifyRecurrenceException(raw, {
      now: ctx.now,
      timezone: ctx.options.timezone,
    }),
  );
  fields.context_hint = reminderContextHint(texts);
  ctx.result.schedule = schedule;

  const hint = fields.context_hint;
  for (const span of found.spans) {
    const field = ROLE_FIELDS[span.role];
    const confidence = spanConfidence?.(span) ?? (field ? FIELD_CONFIDENCE[field] : FIELD_CONFIDENCE.context_hint);
    const value = { start: span.start, end: span.end, text: span.text };
    if (field && hasValue(fields[field])) {
      addFieldSpan(ctx, field, value);
      setFieldConfidence(ctx, field, confidence);
    }
    if (hint && ctx.result.meta.spans.context_hint.length === 0 && span.text.toLowerCase().includes(hint)) {
      addFieldSpan(ctx, 'context_hint', value);
    }
    addDebugStep(ctx, {
      pass,
      ruleId: `schedule.${span.role}`,
      start: span.start,
      end: span.end,
      text: span.text,
      confidence,
      note: `resolved reminder ${span.role.replace(/_/g, ' ')} with kalandar-dates`,
    });
  }

  for (const field of SCHEDULE_FIELDS) {
    if (hasValue(fields[field]) && ctx.result.meta.confidence[field] === 0) {
      setFieldConfidence(ctx, field, FIELD_CONFIDENCE[field]);
    }
  }

  const warnings = scheduleWarnings(found);
  if (warnings.length > 0) {
    ctx.result.warnings = [...(ctx.result.warnings ?? []), ...warnings];
  }
};
