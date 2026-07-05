import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractRecurrence, setFieldConfidence } from '../utils.ts';

export const recurrencePass: ReminderParsePass = (ctx) => {
  const extraction = extractRecurrence(ctx.working);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;
  ctx.state.anchorDayOfMonth = extraction.anchorDayOfMonth;
  ctx.state.anchorMonthDay = extraction.anchorMonthDay;

  if (!extraction.recurrence) {
    return;
  }

  ctx.result.fields.recurrence = extraction.recurrence;
  setFieldConfidence(ctx, 'recurrence', 1);

  if (extraction.span) {
    addFieldSpan(ctx, 'recurrence', extraction.span);
    addDebugStep(ctx, {
      pass: 'recurrencePass',
      ruleId: 'recurrence.extract',
      start: extraction.span.start,
      end: extraction.span.end,
      text: extraction.span.text,
      confidence: 1,
      note: 'extracted reminder recurrence',
    });
  }
};
