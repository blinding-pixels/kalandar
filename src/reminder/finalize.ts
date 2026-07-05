import type { ParseWarning } from '../shared/types.ts';
import { formatDateInTimezone, formatDateTimeInTimezone } from '../shared/utils.ts';
import type { ReminderParseContext, ReminderParseResult } from './types.ts';
import { resolveRecurringReminderAt } from './utils.ts';

const WEEKDAY_NAMES = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

const parseComparableDate = (value: string | null): Date | null => {
  if (!value) {
    return null;
  }
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
};

export const applyReminderRecurrencePostProcess = (ctx: ReminderParseContext): void => {
  const recurrence = ctx.result.fields.recurrence;
  if (!recurrence) {
    return;
  }

  const explicitHour = ctx.state.explicitHour;
  const explicitMinute = ctx.state.explicitMinute;
  const hour = explicitHour ?? 9;
  const minute = explicitMinute ?? 0;

  if (recurrence.frequency === 'daily' && recurrence.interval > 1 && explicitHour === null && explicitMinute === null) {
    ctx.result.fields.remind_at = null;
    return;
  }

  const nowLocal = formatDateTimeInTimezone(ctx.now, ctx.options.timezone);
  const nowDateIso = nowLocal.slice(0, 10);
  const nowMinutes = Number(nowLocal.slice(11, 13)) * 60 + Number(nowLocal.slice(14, 16));
  const recurrenceStartDate = ctx.state.recurrenceStartDate;
  const searchDateIso = recurrenceStartDate && recurrenceStartDate > nowDateIso ? recurrenceStartDate : nowDateIso;
  const searchTimeMinutes = searchDateIso === nowDateIso ? nowMinutes : 0;
  const referenceDateIso = formatDateInTimezone(ctx.now, ctx.options.timezone);

  const resolved = resolveRecurringReminderAt(
    recurrence,
    searchDateIso,
    searchTimeMinutes,
    referenceDateIso,
    hour,
    minute,
    ctx.state.anchorDayOfMonth,
    ctx.state.anchorMonthDay,
  );

  if (resolved) {
    ctx.result.fields.remind_at = resolved;
  }

  if (ctx.result.fields.context_hint && /^morning$/i.test(ctx.result.fields.context_hint) && recurrence.frequency === 'weekly') {
    ctx.result.fields.context_hint = null;
  }
};

export const buildReminderWarnings = (ctx: ReminderParseContext): ParseWarning[] => {
  const warnings: ParseWarning[] = [];

  if (!ctx.result.fields.remind_at) {
    warnings.push({
      code: 'no_remind_time',
      severity: 'warning',
      message: 'No time specified for reminder.',
      fieldHints: ['remind_at'],
      spans: [],
      details: {},
    });
  }

  if (ctx.state.ambiguousTime) {
    warnings.push({
      code: 'ambiguous_time',
      severity: 'info',
      message: 'Time is ambiguous and could be AM or PM.',
      fieldHints: ['remind_at'],
      spans: ctx.result.meta.spans.remind_at,
      details: {},
    });
  }

  const remindAt = parseComparableDate(ctx.result.fields.remind_at);
  if (remindAt && remindAt.getTime() < ctx.now.getTime()) {
    warnings.push({
      code: 'remind_at_past',
      severity: 'warning',
      message: 'Reminder time is in the past.',
      fieldHints: ['remind_at'],
      spans: ctx.result.meta.spans.remind_at,
      details: {
        remind_at: ctx.result.fields.remind_at,
        now: ctx.now.toISOString(),
      },
    });
  }

  const recurrence = ctx.result.fields.recurrence;
  if (recurrence?.frequency === 'weekly' && recurrence.days && recurrence.days.length > 0 && remindAt) {
    const weekday = WEEKDAY_NAMES[remindAt.getDay()];
    if (!recurrence.days.includes(weekday)) {
      warnings.push({
        code: 'recurrence_day_mismatch',
        severity: 'warning',
        message: "Recurrence day doesn't match remind_at date.",
        fieldHints: ['recurrence', 'remind_at'],
        spans: [...ctx.result.meta.spans.recurrence, ...ctx.result.meta.spans.remind_at],
        details: {
          remindAtWeekday: weekday,
          recurrenceDays: recurrence.days,
        },
      });
    }
  }

  return warnings;
};

export const finalizeReminderParseContext = (ctx: ReminderParseContext): ReminderParseResult => {
  applyReminderRecurrencePostProcess(ctx);
  const warnings = buildReminderWarnings(ctx);
  ctx.result.warnings = warnings.length > 0 ? warnings : null;
  ctx.result.meta.maskedInput = ctx.maskedInput;
  return ctx.result;
};
