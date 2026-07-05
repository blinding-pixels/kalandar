import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, detectDuration, setFieldConfidence } from '../utils.ts';

export const durationPass: TaskParsePass = (ctx) => {
  const extraction = detectDuration(ctx.working);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (!extraction.duration || !extraction.span || !extraction.ruleId) {
    return;
  }

  ctx.result.fields.duration = extraction.duration;
  setFieldConfidence(ctx, 'duration', extraction.confidence);
  addFieldSpan(ctx, 'duration', extraction.span);
  addDebugStep(ctx, {
    pass: 'durationPass',
    ruleId: extraction.ruleId,
    start: extraction.span.start,
    end: extraction.span.end,
    text: extraction.span.text,
    confidence: extraction.confidence,
    note: extraction.note,
  });
};
