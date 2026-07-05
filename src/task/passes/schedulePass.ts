import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractSchedule, mergeStructuredRecurrenceExceptions, setFieldConfidence } from '../utils.ts';

export const schedulePass: TaskParsePass = (ctx) => {
  const extraction = extractSchedule(ctx.working, ctx.now);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;
  ctx.state.recurring = extraction.recurring;

  for (const match of extraction.extractions) {
    if (match.field === 'recurrence_exception') {
      ctx.result.fields.recurrence_exception = mergeStructuredRecurrenceExceptions(
        ctx.result.fields.recurrence_exception,
        match.value,
        ctx.now,
        ctx.options.timezone,
      );
    } else {
      ctx.result.fields[match.field] = match.value as never;
    }
    setFieldConfidence(ctx, match.field, match.confidence);
    addFieldSpan(ctx, match.field, match.span);
    addDebugStep(ctx, {
      pass: 'schedulePass',
      ruleId: match.ruleId,
      start: match.span.start,
      end: match.span.end,
      text: match.span.text,
      confidence: match.confidence,
      note: match.note,
    });
  }
};
