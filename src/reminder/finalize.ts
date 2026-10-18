import type { ParseWarning } from '../shared/types.ts';
import type { ReminderParseContext, ReminderParseResult } from './types.ts';

export const buildReminderWarnings = (ctx: ReminderParseContext): ParseWarning[] => {
  if (ctx.result.fields.remind_at || ctx.result.fields.recurrence) {
    return [];
  }

  return [
    {
      code: 'no_remind_time',
      severity: 'warning',
      message: 'No time specified for reminder.',
      fieldHints: ['remind_at'],
      spans: [],
      details: {},
    },
  ];
};

export const finalizeReminderParseContext = (ctx: ReminderParseContext): ReminderParseResult => {
  const warnings = [...buildReminderWarnings(ctx), ...(ctx.result.warnings ?? [])];
  ctx.result.warnings = warnings.length > 0 ? warnings : null;
  ctx.result.meta.maskedInput = ctx.maskedInput;
  return ctx.result;
};
