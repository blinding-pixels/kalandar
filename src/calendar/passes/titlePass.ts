import type { ParseContext } from '../types.ts';
import { extractCalendarContactSuggestions } from '../contactSuggestions.ts';
import { hasStructuredFields, setFieldConfidence, stripEdgeGlueWords } from '../utils.ts';
import { TITLE_SMALL_WORDS } from '../../nlp/shared/constants.ts';
import {
  stripLeadingTitleFiller,
  stripReminderLeadPrefix,
  stripTrailingDanglingPreposition,
} from '../../nlp/shared/title-utils.ts';

const cleanTitleNoise = (input: string): string =>
  input
    .replace(/[()[\]]/g, ' ')
    .replace(/[,;]+/g, ' ')
    .replace(/[–—]+/g, ' ')
    .replace(/\s+-\s+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const trimEdgePrepositions = (input: string): string =>
  input
    .replace(/^(?:on|at|with|from|to|for|and)\s+/i, '')
    .replace(/\s+(?:on|at|with|from|to|for|and)$/i, '')
    .trim();

const stripTrailingTemporalGlue = (input: string): string =>
  input
    .replace(/\s+(?:till|until)\s*[.,;:!?-]*\s*$/i, '')
    .replace(/[.]+$/g, '')
    .trim();

const stripPriorityNoise = (input: string): string =>
  input
    .replace(/\b(?:high|medium|low)\s+priority\b/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const stripMentionNoise = (input: string): string =>
  input
    .replace(/\b(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b/gi, ' ')
    .replace(/@[a-z0-9_.+-]+\b/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const stripTemporalNoise = (input: string): string =>
  input
    .replace(/\bin\s+\d+\s+(?:day|days|week|weeks|month|months)\b/gi, ' ')
    .replace(/\bevery\s+other\s+(?:day|week)\b/gi, ' ')
    .replace(/\b(?:next|this)\s+(?:week|month)\b/gi, ' ')
    .replace(
      /\b(?:today|tomorrow|tonight|tmr|yesterday|starting|weekly|daily|monthly|yearly|annually)\b/gi,
      ' ',
    )
    .replace(/\b(?:morning|afternoon|evening|noon)\b/gi, ' ')
    .replace(/\b(?:end of (?:the )?day|end of (?:the )?week|end of (?:the )?month|eod|eow)\b/gi, ' ')
    .replace(/\b(?:am|pm)\b/gi, ' ')
    .replace(/\b(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)\b/gi, ' ')
    .replace(/\b\d{4}-\d{2}-\d{2}\b/g, ' ')
    .replace(/\bfrom\s+now\b/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const smartCalendarTitleCase = (input: string): string => {
  const tokens = input
    .split(/\s+/)
    .filter(Boolean);

  return tokens
    .map((token, index) => {
      if (/^[A-Z]{2,4}$/.test(token)) {
        return token;
      }
      const lower = token.toLowerCase();
      if (index > 0 && index < tokens.length - 1 && TITLE_SMALL_WORDS.has(lower)) {
        return lower;
      }
      return lower.charAt(0).toUpperCase() + lower.slice(1);
    })
    .join(' ');
};

const titleSpansFromMaskedInput = (ctx: ParseContext): Array<{ start: number; end: number; text: string }> => {
  const spans: Array<{ start: number; end: number; text: string }> = [];
  let rangeStart = -1;

  for (let index = 0; index < ctx.maskedInput.length; index += 1) {
    const char = ctx.maskedInput[index];
    if (/\S/.test(char)) {
      if (rangeStart === -1) {
        rangeStart = index;
      }
      continue;
    }

    if (rangeStart !== -1) {
      const end = index;
      const text = ctx.rawInput.slice(rangeStart, end);
      spans.push({ start: rangeStart, end, text });
      rangeStart = -1;
    }
  }

  if (rangeStart !== -1) {
    const text = ctx.rawInput.slice(rangeStart);
    spans.push({ start: rangeStart, end: ctx.rawInput.length, text });
  }

  return spans;
};

export const titlePass = (ctx: ParseContext): void => {
  const base = cleanTitleNoise(ctx.maskedInput);
  const shouldStripReminderLeadPrefix = /^\s*(?:remind\s+me|don['\u2019]?t\s+forget)\b/i.test(ctx.rawInput);
  const titleBase = shouldStripReminderLeadPrefix ? stripReminderLeadPrefix(base) : base;
  const mentionAndPriorityStripped = stripMentionNoise(stripPriorityNoise(titleBase));
  const temporalStripped = stripTemporalNoise(mentionAndPriorityStripped);
  const temporalContentRemoved = temporalStripped !== mentionAndPriorityStripped || ctx.maskedInput !== ctx.rawInput;
  const title = stripEdgeGlueWords(
    smartCalendarTitleCase(
      stripTrailingDanglingPreposition(
        stripTrailingTemporalGlue(
          stripLeadingTitleFiller(
            trimEdgePrepositions(temporalStripped),
          ),
        ),
        temporalContentRemoved,
      ),
    ),
  );

  if (!title) {
    ctx.result.fields.title = null;
    return;
  }

  ctx.result.fields.title = title;
  const confidence = hasStructuredFields(ctx.result) ? 0.9 : 0.74;
  setFieldConfidence(ctx, 'title', confidence);

  const contactSuggestions = extractCalendarContactSuggestions(title);
  if (contactSuggestions.length > 0) {
    ctx.result.fields.contact_suggestions = contactSuggestions;
    setFieldConfidence(ctx, 'contact_suggestions', 0.78);
    ctx.result.meta.debugSteps.push({
      pass: 'title',
      ruleId: 'title.contact_suggestions',
      start: 0,
      end: ctx.rawInput.length,
      text: contactSuggestions.map((entry) => entry.text).join(', '),
      confidence: 0.78,
      note: 'contact suggestions inferred from title with-phrase',
    });
  }

  for (const span of titleSpansFromMaskedInput(ctx)) {
    ctx.result.meta.spans.title.push({
      start: span.start,
      end: span.end,
      text: span.text,
      ruleId: 'title.leftover',
      confidence,
    });
  }

  ctx.result.meta.debugSteps.push({
    pass: 'title',
    ruleId: 'title.leftover',
    start: 0,
    end: ctx.rawInput.length,
    text: title,
    confidence,
    note: 'title inferred from unmasked leftover text',
  });
};
