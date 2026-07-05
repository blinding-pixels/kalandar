import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, applyNamedDateRules, setFieldConfidence } from '../utils.ts';

export const namedDatePass: ReminderParsePass = (ctx) => {
  const extraction = applyNamedDateRules(ctx.working, ctx.now, ctx.options.timezone, ctx.result.fields.remind_at);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (!extraction.remindAt) {
    return;
  }

  ctx.result.fields.remind_at = extraction.remindAt;
  setFieldConfidence(ctx, 'remind_at', extraction.confidence);

  if (extraction.span) {
    addFieldSpan(ctx, 'remind_at', extraction.span);
    addDebugStep(ctx, {
      pass: 'namedDatePass',
      ruleId: 'date.named',
      start: extraction.span.start,
      end: extraction.span.end,
      text: extraction.span.text,
      confidence: extraction.confidence,
      note: 'resolved named or ordinal date reference',
    });
  }
};
