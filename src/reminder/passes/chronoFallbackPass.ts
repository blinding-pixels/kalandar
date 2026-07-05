import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, chronoFallback, setFieldConfidence } from '../utils.ts';

export const chronoFallbackPass: ReminderParsePass = (ctx) => {
  if (ctx.result.fields.remind_at || ctx.result.fields.recurrence) {
    return;
  }

  const extraction = chronoFallback(ctx.working, ctx.now, ctx.options.timezone);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

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
      pass: 'chronoFallbackPass',
      ruleId: 'time.chrono_fallback',
      start: extraction.span.start,
      end: extraction.span.end,
      text: extraction.span.text,
      confidence: extraction.confidence,
      note: 'resolved time/date using chrono fallback parser',
    });
  }
};
