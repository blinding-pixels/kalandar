import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractProject, setFieldConfidence } from '../utils.ts';

export const projectPass: TaskParsePass = (ctx) => {
  const extraction = extractProject(ctx.working);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (!extraction.project || !extraction.span || !extraction.ruleId) {
    return;
  }

  ctx.result.fields.project = extraction.project;
  setFieldConfidence(ctx, 'project', extraction.confidence);
  addFieldSpan(ctx, 'project', extraction.span);
  addDebugStep(ctx, {
    pass: 'projectPass',
    ruleId: extraction.ruleId,
    start: extraction.span.start,
    end: extraction.span.end,
    text: extraction.span.text,
    confidence: extraction.confidence,
    note: extraction.note,
  });
};
