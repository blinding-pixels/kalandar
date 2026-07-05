import type { TaskParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, extractTags, setFieldConfidence } from '../utils.ts';

export const tagsPass: TaskParsePass = (ctx) => {
  const extraction = extractTags(ctx.working);
  ctx.working = extraction.working;
  ctx.maskedInput = extraction.working;
  ctx.result.meta.maskedInput = extraction.working;

  if (extraction.tags.length === 0) {
    return;
  }

  ctx.result.fields.tags = extraction.tags;
  setFieldConfidence(ctx, 'tags', extraction.confidence);

  for (const span of extraction.spans) {
    addFieldSpan(ctx, 'tags', span);
    addDebugStep(ctx, {
      pass: 'tagsPass',
      ruleId: /#/.test(span.text) ? 'tags.hashtag' : 'tags.clause',
      start: span.start,
      end: span.end,
      text: span.text,
      confidence: extraction.confidence,
      note: /#/.test(span.text) ? 'captured hashtag tag(s)' : 'captured tag clause',
    });
  }
};
