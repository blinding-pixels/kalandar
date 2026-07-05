import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, detectStatus, setFieldConfidence } from '../utils.ts';

export const statusPass: TaskParsePass = (ctx) => {
  const extraction = detectStatus(ctx.working);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (!extraction.status || !extraction.span || !extraction.ruleId) {
    return;
  }

  ctx.result.fields.status = extraction.status;
  setFieldConfidence(ctx, 'status', extraction.confidence);
  addFieldSpan(ctx, 'status', extraction.span);
  addDebugStep(ctx, {
    pass: 'statusPass',
    ruleId: extraction.ruleId,
    start: extraction.span.start,
    end: extraction.span.end,
    text: extraction.span.text,
    confidence: extraction.confidence,
    note: extraction.note,
  });
};
