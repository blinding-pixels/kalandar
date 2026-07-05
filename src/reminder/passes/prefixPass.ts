import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, stripPrefixes } from '../utils.ts';

export const prefixPass: ReminderParsePass = (ctx) => {
  const stripped = stripPrefixes(ctx.working);
  ctx.working = stripped.working;
  ctx.maskedInput = stripped.working;
  ctx.result.meta.maskedInput = stripped.working;

  for (const span of stripped.spans) {
    addDebugStep(ctx, {
      pass: 'prefixPass',
      ruleId: 'prefix.strip',
      start: span.start,
      end: span.end,
      text: span.text,
      confidence: 1,
      note: 'stripped reminder prefix/filler',
    });
  }

  if (ctx.working) {
    addFieldSpan(ctx, 'title', {
      start: 0,
      end: ctx.working.length,
      text: ctx.working,
    });
  }
};
