import { captureSchedule, removeScheduleSpans } from '../../dates/index.ts';
import type { CaptureSchedule, ScheduleRole, ScheduleSpan } from '../../dates/index.ts';
import { classifyRecurrenceException } from '../recurrenceExceptions.ts';
import type { ParseContext, SpanFields } from '../types.ts';
import { addDebugStep, addFieldSpan, setFieldConfidence } from '../utils.ts';

const SCHEDULE_CONFIDENCE = 0.9;

const MOMENT_ROLES: ScheduleRole[] = ['event_start', 'due_date', 'due_time', 'trigger'];

const ROLE_FIELDS: Partial<Record<ScheduleRole, keyof SpanFields>> = {
  duration: 'duration_minutes',
  lead_time: 'alerts',
  alert: 'alerts',
  recurrence: 'recurrence',
  recurrence_start: 'recurrence_start',
  recurrence_end: 'recurrence_end',
  recurrence_exception: 'recurrence_exception',
};

const nonNegative = (value: number | null | undefined): number | null =>
  typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;

const addSpanOnce = (ctx: ParseContext, field: keyof SpanFields, span: ScheduleSpan, start: number, end: number, confidence: number): void => {
  if (ctx.result.meta.spans[field].some((existing) => existing.start === start && existing.end === end)) {
    return;
  }
  addFieldSpan(ctx, field, {
    start,
    end,
    text: span.text.slice(start - span.start, end - span.start),
    ruleId: `schedule.${span.role}`,
    confidence,
  });
};

const addValueSpan = (
  ctx: ParseContext,
  field: keyof SpanFields,
  value: string | null,
  spans: ScheduleSpan[],
  roles: ScheduleRole[],
  confidenceOf: (span: ScheduleSpan) => number,
): void => {
  if (!value) {
    return;
  }
  const needle = value.toLowerCase();
  for (const span of spans) {
    if (!roles.includes(span.role)) {
      continue;
    }
    const offset = span.text.toLowerCase().indexOf(needle);
    if (offset < 0) {
      continue;
    }
    addSpanOnce(ctx, field, span, span.start + offset, span.start + offset + value.length, confidenceOf(span));
    return;
  }
};

const fieldConfidence = (ctx: ParseContext, field: keyof SpanFields, fallback: number): number => {
  const spans = ctx.result.meta.spans[field];
  return spans.length > 0 ? Math.max(...spans.map((span) => span.confidence)) : fallback;
};

export const applyCaptureSchedule = (
  ctx: ParseContext,
  capture: CaptureSchedule,
  confidenceOf: (span: ScheduleSpan) => number = () => SCHEDULE_CONFIDENCE,
): void => {
  const { spans, texts, schedule } = capture;
  const fields = ctx.result.fields;

  fields.date = texts.date;
  fields.time = texts.time;
  fields.end_time = texts.end_time;
  fields.duration_minutes = nonNegative(schedule?.duration_minutes);
  fields.recurrence = texts.recurrence;
  fields.recurrence_start = texts.recurrence_start;
  fields.recurrence_end = texts.recurrence_end;
  fields.recurrence_exception = texts.recurrence_exceptions.map((raw) =>
    classifyRecurrenceException(raw, { now: ctx.now, timezone: ctx.options.timezone }),
  );
  const alertMinutes = nonNegative(schedule?.alert_minutes_before ?? schedule?.lead_minutes);
  fields.alerts = alertMinutes === null ? null : [{ offset_minutes: alertMinutes }];
  ctx.result.schedule = schedule;

  addValueSpan(ctx, 'date', texts.date, spans, MOMENT_ROLES, confidenceOf);
  addValueSpan(ctx, 'time', texts.time, spans, MOMENT_ROLES, confidenceOf);
  addValueSpan(ctx, 'end_time', texts.end_time, spans, [...MOMENT_ROLES, 'event_end'], confidenceOf);

  for (const span of spans) {
    const confidence = confidenceOf(span);
    const field = ROLE_FIELDS[span.role];
    if (field) {
      addSpanOnce(ctx, field, span, span.start, span.end, confidence);
    }
    addDebugStep(ctx, {
      pass: 'schedule',
      ruleId: `schedule.${span.role}`,
      start: span.start,
      end: span.end,
      text: span.text,
      confidence,
      note: `${span.role.replace(/_/g, ' ')} resolved by kalandar-dates`,
    });
  }

  const fallback = spans.length > 0 ? Math.max(...spans.map(confidenceOf)) : SCHEDULE_CONFIDENCE;
  const populated: Array<[keyof SpanFields, boolean]> = [
    ['date', fields.date !== null],
    ['time', fields.time !== null],
    ['end_time', fields.end_time !== null],
    ['duration_minutes', fields.duration_minutes !== null],
    ['recurrence', fields.recurrence !== null],
    ['recurrence_start', fields.recurrence_start !== null],
    ['recurrence_end', fields.recurrence_end !== null],
    ['recurrence_exception', fields.recurrence_exception.length > 0],
    ['alerts', fields.alerts !== null],
  ];
  for (const [field, present] of populated) {
    if (present) {
      setFieldConfidence(ctx, field, fieldConfidence(ctx, field, fallback));
    }
  }

  if (capture.warnings.length > 0) {
    ctx.result.warnings = [
      ...(ctx.result.warnings || []),
      ...capture.warnings.map((warning) => ({
        code: warning.code,
        severity: 'info' as const,
        message: warning.message,
        fieldHints: null,
        spans: null,
        details: null,
      })),
    ];
  }
};

export const schedulePass = (ctx: ParseContext): void => {
  const capture = captureSchedule(ctx.rawInput, 'calendar', {
    now: ctx.now,
    timezone: ctx.options.timezone,
    locale: ctx.options.locale,
  });
  applyCaptureSchedule(ctx, capture);

  const remaining = removeScheduleSpans(ctx.maskedInput, capture.spans);
  ctx.rawInput = remaining;
  ctx.maskedInput = remaining;
  ctx.result.meta.maskedInput = remaining;
};
