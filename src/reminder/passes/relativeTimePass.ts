import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, applyRelativeTimeRules, setFieldConfidence } from '../utils.ts';

export const relativeTimePass: ReminderParsePass = (ctx) => {
  if (ctx.result.fields.remind_at) {
    return;
  }

  const extraction = applyRelativeTimeRules(ctx.working, ctx.now, ctx.options.timezone);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (extraction.contextHint) {
    ctx.result.fields.context_hint = extraction.contextHint;
    setFieldConfidence(ctx, 'context_hint', extraction.remindAt ? 0.9 : 0.6);
    if (extraction.span) {
      addFieldSpan(ctx, 'context_hint', extraction.span);
    }
  }

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
      pass: 'relativeTimePass',
      ruleId: 'time.relative',
      start: extraction.span.start,
      end: extraction.span.end,
      text: extraction.span.text,
      confidence: extraction.confidence,
      note: 'resolved relative reminder time',
    });
  }
};
