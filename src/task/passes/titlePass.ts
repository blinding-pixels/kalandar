import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, buildTaskTitle, setFieldConfidence } from '../utils.ts';

export const titlePass: TaskParsePass = (ctx) => {
  const built = buildTaskTitle(ctx.working);
  ctx.result.fields.title = built.title;
  setFieldConfidence(ctx, 'title', built.confidence);

  addFieldSpan(ctx, 'title', {
    start: 0,
    end: built.title.length,
    text: built.title,
  });

  addDebugStep(ctx, {
    pass: 'titlePass',
    ruleId: 'title.finalize',
    start: 0,
    end: built.title.length,
    text: built.title,
    confidence: built.confidence,
    note: built.title === 'Task' ? 'fallback task title used because little text remained' : 'built title from residual working text',
  });
};
