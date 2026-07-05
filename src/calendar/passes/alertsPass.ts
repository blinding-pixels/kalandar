import type { ParseContext } from '../types.ts';
import {
  addDebugStep,
  addFieldSpan,
  isSpanAvailable,
  maskMatch,
  parseDurationToMinutes,
  parseNumberToken,
  setFieldConfidence,
} from '../utils.ts';

const PREFIX_REGEX = /\b(?:remind(?:\s+me)?|alert)\b/gi;

const AMOUNT_UNIT_REGEX =
  /(\d+(?:\.\d+)?|a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s*(minutes?|mins?|min|m|hours?|hrs?|hr|h|days?|day|d)\s+before/gi;
const LOOSE_AMOUNT_UNIT_REGEX =
  /(\d+(?:\.\d+)?|a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s*(minutes?|mins?|min|m|hours?|hrs?|hr|h|days?|day|d)\b/gi;

const DAY_BEFORE_REGEX = /\b(?:the\s+day\s+before|a\s+day\s+before|an\s+day\s+before)\b/gi;
const TOMORROW_MORNING_REGEX = /\btomorrow\s+morning\b/gi;

const findClauseEnd = (input: string, start: number): number => {
  for (let index = start; index < input.length; index += 1) {
    const char = input[index];
    if (char === '.' || char === ',' || char === ';') {
      return index;
    }
  }
  return input.length;
};

const looksLikeRelativeDateAmount = (source: string, localStart: number, localEnd: number): boolean => {
  const before = source.slice(Math.max(0, localStart - 12), localStart).toLowerCase();
  const after = source.slice(localEnd, localEnd + 16).toLowerCase();
  if (/\bin\s*$/.test(before) || /\bafter\s*$/.test(before)) {
    return true;
  }
  if (/^\s+from\s+now\b/.test(after)) {
    return true;
  }
  return false;
};

export const alertsPass = (ctx: ParseContext): void => {
  const prefixMatches = Array.from(ctx.maskedInput.matchAll(PREFIX_REGEX));

  for (const prefixMatch of prefixMatches) {
    const clauseStart = prefixMatch.index ?? -1;
    if (clauseStart < 0) {
      continue;
    }

    const clauseEnd = findClauseEnd(ctx.maskedInput, clauseStart);
    if (!isSpanAvailable(ctx, clauseStart, clauseEnd)) {
      continue;
    }

    const clauseText = ctx.maskedInput.slice(clauseStart, clauseEnd);
    const offsetEntries: Array<{ offset: number; start: number }> = [];
    const spans: Array<{ start: number; end: number; text: string; confidence: number; ruleId: string }> = [];

    AMOUNT_UNIT_REGEX.lastIndex = 0;
    for (const amountMatch of clauseText.matchAll(AMOUNT_UNIT_REGEX)) {
      const amountToken = amountMatch[1] || '';
      const unitToken = amountMatch[2] || '';
      const minutes = parseDurationToMinutes(amountToken, unitToken);
      if (!minutes || minutes <= 0) {
        continue;
      }

      const localStart = amountMatch.index ?? 0;
      const spanStart = clauseStart + localStart;
      const spanText = amountMatch[0];
      const spanEnd = spanStart + spanText.length;

      offsetEntries.push({ offset: -minutes, start: spanStart });
      spans.push({
        start: spanStart,
        end: spanEnd,
        text: spanText,
        confidence: 0.92,
        ruleId: 'alerts.relative_before',
      });
    }

    if (/\bbefore\b/i.test(clauseText)) {
      LOOSE_AMOUNT_UNIT_REGEX.lastIndex = 0;
      for (const amountMatch of clauseText.matchAll(LOOSE_AMOUNT_UNIT_REGEX)) {
        const amountToken = amountMatch[1] || '';
        const unitToken = amountMatch[2] || '';
        const minutes = parseDurationToMinutes(amountToken, unitToken);
        if (!minutes || minutes <= 0) {
          continue;
        }

      const localStart = amountMatch.index ?? 0;
      const spanStart = clauseStart + localStart;
      const spanText = amountMatch[0];
      const spanEnd = spanStart + spanText.length;
      if (looksLikeRelativeDateAmount(clauseText, localStart, localStart + spanText.length)) {
        continue;
      }
      if (spans.some((entry) => entry.start === spanStart && entry.end === spanEnd)) {
        continue;
      }
        if (spanText.trim().toLowerCase() === 'and') {
          continue;
        }

        offsetEntries.push({ offset: -minutes, start: spanStart });
        spans.push({
          start: spanStart,
          end: spanEnd,
          text: spanText,
          confidence: 0.8,
          ruleId: 'alerts.relative_before_loose',
        });
      }
    }

    DAY_BEFORE_REGEX.lastIndex = 0;
    for (const dayBeforeMatch of clauseText.matchAll(DAY_BEFORE_REGEX)) {
      const localStart = dayBeforeMatch.index ?? 0;
      const spanText = dayBeforeMatch[0];
      const spanStart = clauseStart + localStart;
      const spanEnd = spanStart + spanText.length;
      offsetEntries.push({ offset: -1440, start: spanStart });
      spans.push({
        start: spanStart,
        end: spanEnd,
        text: spanText,
        confidence: 0.88,
        ruleId: 'alerts.day_before',
      });
    }

    if (spans.length === 0) {
      const tomorrowMatch = TOMORROW_MORNING_REGEX.exec(clauseText);
      TOMORROW_MORNING_REGEX.lastIndex = 0;
      if (tomorrowMatch) {
        const localStart = tomorrowMatch.index ?? 0;
        const spanText = tomorrowMatch[0];
        const spanStart = clauseStart + localStart;
        const spanEnd = spanStart + spanText.length;
        offsetEntries.push({ offset: -1440, start: spanStart });
        spans.push({
          start: spanStart,
          end: spanEnd,
          text: spanText,
          confidence: 0.52,
          ruleId: 'alerts.tomorrow_morning',
        });
      }
    }

    if (spans.length === 0) {
      LOOSE_AMOUNT_UNIT_REGEX.lastIndex = 0;
      const looseMatch = LOOSE_AMOUNT_UNIT_REGEX.exec(clauseText);
      LOOSE_AMOUNT_UNIT_REGEX.lastIndex = 0;
      if (looseMatch?.index !== undefined) {
        const minutes = parseDurationToMinutes(looseMatch[1] || '', looseMatch[2] || '');
        if (minutes && minutes > 0) {
          const localStart = looseMatch.index;
          const spanText = looseMatch[0];
          if (looksLikeRelativeDateAmount(clauseText, localStart, localStart + spanText.length)) {
            continue;
          }
          const spanStart = clauseStart + looseMatch.index;
          const spanEnd = spanStart + spanText.length;
          if (spanText.trim().toLowerCase() === 'and') {
            continue;
          }
          offsetEntries.push({ offset: -minutes, start: spanStart });
          spans.push({
            start: spanStart,
            end: spanEnd,
            text: spanText,
            confidence: 0.66,
            ruleId: 'alerts.loose_amount_before_assumed',
          });
        }
      }
    }

    if (spans.length === 0) {
      continue;
    }

    const uniqueOffsets: number[] = [];
    for (const entry of offsetEntries.sort((left, right) => left.start - right.start)) {
      if (!uniqueOffsets.includes(entry.offset)) {
        uniqueOffsets.push(entry.offset);
      }
    }
    const existingAlerts = ctx.result.fields.alerts || [];
    for (const offset of uniqueOffsets) {
      if (!existingAlerts.some((alert) => alert.offset_minutes === offset)) {
        existingAlerts.push({ offset_minutes: offset });
      }
    }
    ctx.result.fields.alerts = existingAlerts;

    for (const span of spans) {
      addFieldSpan(ctx, 'alerts', {
        start: span.start,
        end: span.end,
        text: span.text,
        ruleId: span.ruleId,
        confidence: span.confidence,
      });
      addDebugStep(ctx, {
        pass: 'alerts',
        ruleId: span.ruleId,
        start: span.start,
        end: span.end,
        text: span.text,
        confidence: span.confidence,
        note: 'detected reminder offset before event',
      });
      setFieldConfidence(ctx, 'alerts', span.confidence);
    }

    const maskEnd = Math.max(
      clauseStart + (prefixMatch[0]?.length || 0),
      ...spans.map((span) => span.end),
    );
    maskMatch(ctx, {
      start: clauseStart,
      end: maskEnd,
      text: ctx.rawInput.slice(clauseStart, maskEnd),
      ruleId: 'alerts.clause',
      confidence: 0.8,
    });
  }

  // Edge case: standalone "30m before" without prefix, only if no alerts already found.
  if (ctx.result.fields.alerts === null) {
    const fallback = /\b(\d+(?:\.\d+)?)\s*(m|min|mins|minutes?)\s+before\b/i.exec(ctx.maskedInput);
    if (fallback?.index !== undefined) {
      const minutes = parseNumberToken(fallback[1]);
      if (minutes !== null && minutes > 0) {
        const start = fallback.index;
        const text = fallback[0];
        const end = start + text.length;
        ctx.result.fields.alerts = [{ offset_minutes: -Math.round(minutes) }];
        addFieldSpan(ctx, 'alerts', {
          start,
          end,
          text,
          ruleId: 'alerts.fallback_before',
          confidence: 0.48,
        });
        addDebugStep(ctx, {
          pass: 'alerts',
          ruleId: 'alerts.fallback_before',
          start,
          end,
          text,
          confidence: 0.48,
          note: 'fallback reminder phrase without explicit remind keyword',
        });
        setFieldConfidence(ctx, 'alerts', 0.48);
        maskMatch(ctx, {
          start,
          end,
          text,
          ruleId: 'alerts.fallback_before',
          confidence: 0.48,
        });
      }
    }
  }
};
