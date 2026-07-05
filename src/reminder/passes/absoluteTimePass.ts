import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, applyAbsoluteTimeRules, setFieldConfidence } from '../utils.ts';

export const absoluteTimePass: ReminderParsePass = (ctx) => {
  if (ctx.result.fields.remind_at) {
    return;
  }

  const extraction = applyAbsoluteTimeRules(ctx.working, ctx.now, ctx.options.timezone);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;
  ctx.state.ambiguousTime = extraction.ambiguousTime;

  if (!extraction.remindAt) {
    return;
  }

  ctx.result.fields.remind_at = extraction.remindAt;

  ctx.state.explicitHour = extraction.explicitHour;
  ctx.state.explicitMinute = extraction.explicitMinute;
  setFieldConfidence(ctx, 'remind_at', extraction.confidence);

  if (extraction.span) {
    addFieldSpan(ctx, 'remind_at', extraction.span);
    addDebugStep(ctx, {
      pass: 'absoluteTimePass',
      ruleId: 'time.absolute',
      start: extraction.span.start,
      end: extraction.span.end,
      text: extraction.span.text,
      confidence: extraction.confidence,
      note: 'resolved explicit clock time',
    });
  }
};
