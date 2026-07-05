import type { TaskParsePass } from '../types.ts';
import { addDebugStep, applyDateTypoCorrections } from '../utils.ts';

export const dateTypoCorrectionPass: TaskParsePass = (ctx) => {
  const correction = applyDateTypoCorrections(ctx.working);
  ctx.working = correction.working;
  ctx.maskedInput = correction.working;
  ctx.result.meta.maskedInput = correction.working;

  for (const entry of correction.corrections) {
    addDebugStep(ctx, {
      pass: 'dateTypoCorrectionPass',
      ruleId: 'date_typo.correction',
      start: entry.start,
      end: entry.end,
      text: entry.text,
      confidence: 0.9,
      note: `normalized "${entry.text}" to "${entry.replacement}" before date parsing`,
    });
  }
};
