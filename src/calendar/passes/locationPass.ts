import type { ParseContext } from '../types.ts';
import { addDebugStep, addFieldSpan, maskMatch, setFieldConfidence } from '../utils.ts';

const PREFIX_REGEX = /(?:^|\s)(at|in|on)\b|@/gi;
const STOP_REGEX =
  /\s+\b(?:with|every|remind|alert|on|from|to|starting|start|for|until|ending|except|invite|w\/|at|in)\b|[,.;()]/i;
const VIRTUAL_LOCATION_REGEX = /^(?:zoom|google\s+meet|meet|teams|webex|slack|discord)\b/i;

const isTimeLike = (value: string): boolean => {
  const candidate = value.trim().toLowerCase();
  if (!candidate) {
    return true;
  }
  if (/^(?:noon|midnight|morning|afternoon|evening|night|today|tomorrow|tonight|next|this)\b/.test(candidate)) {
    return true;
  }
  if (/^(?:in\s+)?\d+\s+(?:day|days|week|weeks|month|months|year|years)\b/.test(candidate)) {
    return true;
  }
  if (/^(?:a|an|one|two|three|four|five|six|seven|eight|nine|ten)\s+(?:day|days|week|weeks|month|months|year|years)\b/.test(candidate)) {
    return true;
  }
  if (/^\d{1,2}(?::\d{2})?\s*(?:am|pm)?$/.test(candidate)) {
    return true;
  }
  if (/^\d{1,2}:\d{2}\b/.test(candidate)) {
    return true;
  }
  if (
    /^\d{1,2}\s+[a-z]/.test(candidate) &&
    !/^\d+\s+(?:main|room|suite|floor|st|street|ave|avenue|rd|road|blvd|boulevard|park|hall)\b/.test(candidate)
  ) {
    return true;
  }
  if (/^\d{1,2}\s+\d/.test(candidate)) {
    return true;
  }
  if (/^\d{1,2}(?::\d{2})?\s*(?:am|pm)\b/.test(candidate)) {
    return true;
  }
  if (/^\d{1,2}\s*(?:-|–|to)\s*\d{1,2}(?::\d{2})?/.test(candidate)) {
    return true;
  }
  if (/^\d{4}-\d{2}-\d{2}$/.test(candidate)) {
    return true;
  }
  if (/^\d+\s+(?:day|days|week|weeks|month|months|year|years)\b/.test(candidate)) {
    return true;
  }
  if (/^(?:in|after)\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b/.test(candidate)) {
    return true;
  }
  return false;
};

const cleanupLocation = (value: string): string =>
  value
    .trim()
    .replace(/^["']+|["']+$/g, '')
    .replace(/\s+/g, ' ')
    .replace(/[\s-]+$/, '')
    .trim();

const hasStrongLocationSignal = (value: string): boolean =>
  /[A-Z]/.test(value) || /\d/.test(value) || /@/.test(value) || VIRTUAL_LOCATION_REGEX.test(value);

const WEEKDAY_WORD_REGEX =
  /\b(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/i;

export const locationPass = (ctx: ParseContext): void => {
  const matches = Array.from(ctx.maskedInput.matchAll(PREFIX_REGEX));

  for (const prefixMatch of matches) {
    const matchIndex = prefixMatch.index ?? -1;
    if (matchIndex < 0) {
      continue;
    }

    const wordPrefix = prefixMatch[1] || '';
    const prefixText = wordPrefix || '@';
    const prefixStart = wordPrefix ? matchIndex + prefixMatch[0].length - wordPrefix.length : matchIndex;
    if (prefixStart < 0) {
      continue;
    }
    const valueStart = prefixStart + prefixText.length;

    if (!/\s/.test(ctx.maskedInput[valueStart] || '')) {
      continue;
    }

    const remainder = ctx.maskedInput.slice(valueStart).replace(/^\s+/, '');
    const leadingSpaces = ctx.maskedInput.slice(valueStart).length - remainder.length;
    const normalizedValueStart = valueStart + leadingSpaces;
    const stopMatch = STOP_REGEX.exec(remainder);
    STOP_REGEX.lastIndex = 0;

    const rawCandidate = stopMatch ? remainder.slice(0, stopMatch.index) : remainder;
    const cleaned = cleanupLocation(rawCandidate);

    if (!cleaned || cleaned.length > 80 || isTimeLike(cleaned)) {
      continue;
    }

    if (prefixText.toLowerCase() === 'on' && !VIRTUAL_LOCATION_REGEX.test(cleaned)) {
      continue;
    }

    if (prefixText.toLowerCase() === 'in' && !hasStrongLocationSignal(cleaned)) {
      continue;
    }

    if (prefixText.toLowerCase() === 'in' && (/^the\s+/i.test(cleaned) || WEEKDAY_WORD_REGEX.test(cleaned))) {
      continue;
    }

    const spanStart = prefixStart;
    const spanEnd = normalizedValueStart + rawCandidate.length;
    const spanText = ctx.rawInput.slice(spanStart, spanEnd);
    const quoted = /["']/.test(rawCandidate);
    const virtual = VIRTUAL_LOCATION_REGEX.test(cleaned);
    const confidence = quoted ? 0.82 : virtual ? 0.78 : 0.68;

    if (ctx.result.fields.location && confidence < ctx.result.meta.confidence.location) {
      continue;
    }

    ctx.result.fields.location = cleaned;
    addFieldSpan(ctx, 'location', {
      start: spanStart,
      end: spanEnd,
      text: spanText,
      ruleId: 'location.preposition_phrase',
      confidence,
    });
    setFieldConfidence(ctx, 'location', confidence);
    addDebugStep(ctx, {
      pass: 'location',
      ruleId: 'location.preposition_phrase',
      start: spanStart,
      end: spanEnd,
      text: spanText,
      confidence,
      note: 'location extracted from at/in phrase',
    });

    maskMatch(ctx, {
      start: spanStart,
      end: spanEnd,
      text: spanText,
      ruleId: 'location.preposition_phrase',
      confidence,
    });
  }
};
