import type { EventParseResult, ParseContext, ParseWarning } from './types.ts';

export const finalizeCalendarParseContext = (ctx: ParseContext): EventParseResult => {
  const { date, time, recurrence } = ctx.result.fields;
  const warnings: ParseWarning[] = [];
  if (!date && !time && !recurrence) {
    warnings.push({
      code: 'calendar-time-missing',
      severity: 'warning',
      message: 'No date, time, or recurrence extracted.',
      fieldHints: ['date', 'time', 'recurrence'],
      spans: null,
      details: null,
    });
  }
  warnings.push(...(ctx.result.warnings || []));

  ctx.result.meta.maskedInput = ctx.maskedInput;
  ctx.result.warnings = warnings.length > 0 ? warnings : null;
  return ctx.result;
};
