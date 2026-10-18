import { captureSchedule, removeScheduleSpans } from '../../dates/index.ts';
import type { CaptureSchedule, ScheduleRole, ScheduleTexts } from '../../dates/index.ts';
import type { TaskParseContext, TaskParsePass, TaskParseResult } from '../types.ts';
import { addDebugStep, addFieldSpan, mergeStructuredRecurrenceExceptions, setFieldConfidence } from '../utils.ts';

type ScheduleField = Extract<
  keyof TaskParseResult['meta']['spans'],
  'due_date' | 'due_time' | 'duration' | 'recurrence' | 'recurrence_start' | 'recurrence_end' | 'recurrence_exception'
>;

const SCHEDULE_CONFIDENCE = 0.95;
const MOMENT_ROLES: ScheduleRole[] = ['due_date', 'due_time', 'event_start', 'trigger'];
const ROLE_FIELDS: Partial<Record<ScheduleRole, ScheduleField>> = {
  duration: 'duration',
  recurrence: 'recurrence',
  recurrence_start: 'recurrence_start',
  recurrence_end: 'recurrence_end',
  recurrence_exception: 'recurrence_exception',
};

const fieldsForRole = (role: ScheduleRole, texts: ScheduleTexts): ScheduleField[] => {
  if (MOMENT_ROLES.includes(role)) {
    const fields: ScheduleField[] = [];
    if (texts.date) {
      fields.push('due_date');
    }
    if (texts.time) {
      fields.push('due_time');
    }
    return fields;
  }
  const field = ROLE_FIELDS[role];
  if (!field) {
    return [];
  }
  if (field === 'recurrence_exception') {
    return texts.recurrence_exceptions.length > 0 ? [field] : [];
  }
  return texts[field] ? [field] : [];
};

export const applyCaptureSchedule = (ctx: TaskParseContext, found: CaptureSchedule, confidence = SCHEDULE_CONFIDENCE): void => {
  const { texts } = found;
  ctx.result.fields.due_date = texts.date;
  ctx.result.fields.due_time = texts.time;
  ctx.result.fields.duration = texts.duration;
  ctx.result.fields.recurrence = texts.recurrence;
  ctx.result.fields.recurrence_start = texts.recurrence_start;
  ctx.result.fields.recurrence_end = texts.recurrence_end;
  for (const raw of texts.recurrence_exceptions) {
    ctx.result.fields.recurrence_exception = mergeStructuredRecurrenceExceptions(
      ctx.result.fields.recurrence_exception,
      raw,
      ctx.now,
      ctx.options.timezone,
    );
  }
  ctx.state.recurring = Boolean(texts.recurrence);
  ctx.result.schedule = found.schedule;

  for (const span of found.spans) {
    const fields = fieldsForRole(span.role, texts);
    for (const field of fields) {
      setFieldConfidence(ctx, field, confidence);
      addFieldSpan(ctx, field, span);
    }
    addDebugStep(ctx, {
      pass: 'schedulePass',
      ruleId: `schedule.${span.role}`,
      start: span.start,
      end: span.end,
      text: span.text,
      confidence,
      note: `resolved ${span.role.replace(/_/g, ' ')} span with kalandar-dates`,
    });
  }

  if (found.warnings.length > 0) {
    ctx.result.warnings = [
      ...(ctx.result.warnings || []),
      ...found.warnings.map((warning) => ({
        code: warning.code,
        severity: 'info' as const,
        message: warning.message,
        fieldHints: [],
        spans: [],
        details: {},
      })),
    ];
  }
};

export const schedulePass: TaskParsePass = (ctx) => {
  const found = captureSchedule(ctx.working, 'task', {
    now: ctx.now,
    timezone: ctx.options.timezone,
    locale: ctx.options.locale,
  });
  applyCaptureSchedule(ctx, found);
  ctx.working = removeScheduleSpans(ctx.working, found.spans);
  ctx.maskedInput = ctx.working;
  ctx.result.meta.maskedInput = ctx.working;
};
