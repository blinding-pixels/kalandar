import type { ParseContext } from '../types.ts';
import {
  addDebugStep,
  addFieldSpan,
  isSpanAvailable,
  maskMatch,
  parseDurationToMinutes,
  setFieldConfidence,
} from '../utils.ts';

interface DurationRule {
  id: string;
  regex: RegExp;
  confidence: number;
  extract: (match: RegExpMatchArray) => number | null;
  note: string;
}

const DURATION_RULES: DurationRule[] = [
  {
    id: 'duration.for_numeric_unit',
    regex: /\bfor\s+(\d+(?:\.\d+)?)\s*(days?|d|hours?|hrs?|hr|h|minutes?|mins?|min|m)\b/gi,
    confidence: 0.94,
    extract: (match) => parseDurationToMinutes(match[1] || '', match[2] || ''),
    note: 'explicit numeric duration after for',
  },
  {
    id: 'duration.for_article_hour',
    regex: /\bfor\s+(?:an|a|one)\s+hour\b/gi,
    confidence: 0.92,
    extract: () => 60,
    note: 'article hour duration phrase',
  },
  {
    id: 'duration.for_half_hour',
    regex: /\bfor\s+half\s+an?\s+hour\b/gi,
    confidence: 0.88,
    extract: () => 30,
    note: 'half hour duration phrase',
  },
  {
    id: 'duration.for_article_day',
    regex: /\bfor\s+(?:a|an|one)\s+day\b/gi,
    confidence: 0.82,
    extract: () => 1440,
    note: 'article day duration phrase',
  },
];

export const durationPass = (ctx: ParseContext): void => {
  for (const rule of DURATION_RULES) {
    const matches = Array.from(ctx.maskedInput.matchAll(rule.regex));

    for (const match of matches) {
      const fullText = match[0];
      const start = match.index ?? -1;
      const end = start + fullText.length;

      if (!isSpanAvailable(ctx, start, end)) {
        continue;
      }

      const minutes = rule.extract(match);
      if (!minutes || minutes <= 0) {
        continue;
      }

      const roundedMinutes = Math.round(minutes);
      if (ctx.result.fields.duration_minutes === null || rule.confidence >= ctx.result.meta.confidence.duration_minutes) {
        ctx.result.fields.duration_minutes = roundedMinutes;
      }

      addFieldSpan(ctx, 'duration_minutes', {
        start,
        end,
        text: fullText,
        ruleId: rule.id,
        confidence: rule.confidence,
      });
      setFieldConfidence(ctx, 'duration_minutes', rule.confidence);
      addDebugStep(ctx, {
        pass: 'duration',
        ruleId: rule.id,
        start,
        end,
        text: fullText,
        confidence: rule.confidence,
        note: rule.note,
      });
      maskMatch(ctx, {
        start,
        end,
        text: fullText,
        ruleId: rule.id,
        confidence: rule.confidence,
      });
    }
  }
};
