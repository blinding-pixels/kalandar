import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractAssignees, setFieldConfidence } from '../utils.ts';

export const assigneePass: TaskParsePass = (ctx) => {
  const extraction = extractAssignees(ctx.working, ctx.options.knownAssignees);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (extraction.assignees.length === 0) {
    return;
  }

  ctx.result.fields.assignee = extraction.assignees;
  const confidence = extraction.assignees.some((hint) => hint.startsWith('@')) ? 1 : 0.82;
  setFieldConfidence(ctx, 'assignee', confidence);

  for (const span of extraction.spans) {
    addFieldSpan(ctx, 'assignee', span);
    addDebugStep(ctx, {
      pass: 'assigneePass',
      ruleId: span.text.startsWith('@') ? 'assignee.mention' : 'assignee.clause',
      start: span.start,
      end: span.end,
      text: span.text,
      confidence,
      note: span.text.startsWith('@') ? 'captured explicit @mention assignee' : 'captured assignee from directed phrase',
    });
  }
};
