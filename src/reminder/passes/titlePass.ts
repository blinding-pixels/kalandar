import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractTitle, setFieldConfidence } from '../utils.ts';

export const titlePass: ReminderParsePass = (ctx) => {
  const title = extractTitle(ctx.working);
  ctx.result.fields.title = title;

  const tokenCount = title.split(/\s+/).filter(Boolean).length;
  setFieldConfidence(ctx, 'title', tokenCount >= 3 ? 1 : tokenCount >= 2 ? 0.6 : 0.4);

  addFieldSpan(ctx, 'title', {
    start: 0,
    end: ctx.working.length,
    text: ctx.working,
  });

  addDebugStep(ctx, {
    pass: 'titlePass',
    ruleId: 'title.finalize',
    start: 0,
    end: ctx.working.length,
    text: ctx.working,
    confidence: tokenCount >= 3 ? 1 : tokenCount >= 2 ? 0.6 : 0.4,
    note: title === 'Reminder' ? 'fallback reminder title used' : 'constructed title from remaining text',
  });
};
