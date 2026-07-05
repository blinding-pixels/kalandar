import type { ReminderParsePass } from '../types.ts';
import { addDebugStep, addFieldSpan, resolveRecurrenceStartDate, setFieldConfidence } from '../utils.ts';
import { normalizeWhitespace } from '../../shared/utils.ts';
import { classifyRecurrenceException } from '../../calendar/recurrenceExceptions.ts';

const MONTH =
  '(?:january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sep(?:t)?|october|oct|november|nov|december|dec)';
const WEEKDAY =
  '(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues?|wed(?:nesday)?|thu(?:rs?|rsday|ursday)?|fri|sat|sun)';
const DATE_FRAGMENT =
  `(?:today|tomorrow|tonight|next\\s+(?:week|month|year|${WEEKDAY})|this\\s+(?:week|month|year|${WEEKDAY})|\\d{1,2}(?:st|nd|rd|th)?\\s+${MONTH}|${MONTH}\\s+\\d{1,2}(?:st|nd|rd|th)?|end of (?:the )?(?:week|month)|the week of the \\d{1,2}(?:st|nd|rd|th)?)`;

const TIME_CONTEXTS: Record<string, { hour: number; minute: number }> = {
  morning: { hour: 9, minute: 0 },
  noon: { hour: 12, minute: 0 },
  afternoon: { hour: 15, minute: 0 },
  evening: { hour: 18, minute: 0 },
  night: { hour: 20, minute: 0 },
};

const ACTION_TAIL_VERB =
  '(?:add|attach|bring|buy|call|check|clean|email|file|finish|fix|get|mail|pay|pick|plan|prep|prepare|refill|review|send|submit|water)';
const ACTION_TAIL_REGEX = new RegExp(`\\s+(?=(?:and)\\s+${ACTION_TAIL_VERB}\\b)`, 'i');
const EXCEPTION_BOUNDARY_PATTERNS: RegExp[] = [
  /\s+(?=(?:but\s+(?:skip|not)\b|unless\b|except(?:\s+for)?\b))/i,
  /\s+(?=(?:starting|beginning|from)\b)/i,
  /\s+(?=for\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b)/i,
  /\s+(?=(?:until|till|through|ending(?:\s+on)?)\b)/i,
  ACTION_TAIL_REGEX,
  /\s*[.,;:!?](?=\s|$)/i,
];
const TRAILING_CONTEXT_HINT_REGEX = /\b(?:at\s+)?(morning|noon|afternoon|evening|night)\b[.,;:!?]*\s*$/i;

const EXCEPTION_RULES: Array<{ id: string; regex: RegExp; note: string }> = [
  {
    id: 'schedule.recurrence_exception.unless',
    regex: /\bunless\b/i,
    note: 'extracted reminder recurrence exception from unless clause',
  },
  {
    id: 'schedule.recurrence_exception.except',
    regex: /\bexcept(?:\s+for)?\b/i,
    note: 'extracted reminder recurrence exception from except clause',
  },
  {
    id: 'schedule.recurrence_exception.but_skip',
    regex: /\bbut\s+skip\b/i,
    note: 'extracted reminder recurrence exception from but-skip clause',
  },
  {
    id: 'schedule.recurrence_exception.but_not',
    regex: /\bbut\s+not\b/i,
    note: 'extracted reminder recurrence exception from but-not clause',
  },
  {
    id: 'schedule.recurrence_exception.skip_first',
    regex: /\bskip\s+the\s+first\s+one\b/i,
    note: 'extracted explicit skip-the-first reminder recurrence exception',
  },
];

const syncMaskedInput = (ctx: Parameters<ReminderParsePass>[0]): void => {
  ctx.maskedInput = normalizeWhitespace(ctx.working);
  ctx.result.meta.maskedInput = ctx.maskedInput;
};

const extractBoundedClause = (working: string, regex: RegExp): { start: number; end: number; text: string } | null => {
  const match = working.match(regex);
  if (!match || match.index == null) {
    return null;
  }

  const start = match.index;
  const searchStart = start + match[0].length;
  let end = working.length;

  for (const boundary of EXCEPTION_BOUNDARY_PATTERNS) {
    const boundaryMatch = working.slice(searchStart).match(boundary);
    if (!boundaryMatch || boundaryMatch.index == null) {
      continue;
    }
    end = Math.min(end, searchStart + boundaryMatch.index);
  }

  const trailingPunctuation = working.slice(end).match(/^\s*[.,;:!?]+/);
  if (trailingPunctuation) {
    end += trailingPunctuation[0].length;
  }

  const text = normalizeWhitespace(working.slice(start, end).replace(/[.,;:!?]+$/g, ''));
  if (!text) {
    return null;
  }

  return { start, end, text };
};

export const scheduleDetailsPass: ReminderParsePass = (ctx) => {
  const consume = (
    field: 'recurrence_start' | 'recurrence_end' | 'recurrence_window' | 'location' | 'context_hint',
    regex: RegExp,
    confidence: number,
    note: string,
  ): string | null => {
    const match = ctx.working.match(regex);
    if (!match || !match[0]) {
      return null;
    }

    const start = match.index ?? 0;
    const end = start + match[0].length;
    const text = normalizeWhitespace(match[0]);
    const value = text;
    ctx.working = normalizeWhitespace(`${ctx.working.slice(0, start)} ${ctx.working.slice(end)}`);

    ctx.result.fields[field] = value;
    if (field === 'recurrence_start') {
      ctx.state.recurrenceStartDate = resolveRecurrenceStartDate(value, ctx.now, ctx.options.timezone);
    }
    setFieldConfidence(ctx, field, confidence);
    addFieldSpan(ctx, field, { start, end, text: value });
    addDebugStep(ctx, {
      pass: 'scheduleDetailsPass',
      ruleId: `schedule.${field}`,
      start,
      end,
      text: value,
      confidence,
      note,
    });
    syncMaskedInput(ctx);
    return value;
  };

  const pushException = (
    raw: string,
    start: number,
    end: number,
    ruleId: string,
    note: string,
    confidence: number,
  ): void => {
    if (!raw) {
      return;
    }

    const current = ctx.result.fields.recurrence_exception;
    if (!current.some((entry) => entry.raw === raw)) {
      current.push(
        classifyRecurrenceException(raw, {
          now: ctx.now,
          timezone: ctx.options.timezone,
        }),
      );
    }

    setFieldConfidence(ctx, 'recurrence_exception', confidence);
    addFieldSpan(ctx, 'recurrence_exception', { start, end, text: raw });
    addDebugStep(ctx, {
      pass: 'scheduleDetailsPass',
      ruleId,
      start,
      end,
      text: raw,
      confidence,
      note,
    });
    syncMaskedInput(ctx);
  };

  if (ctx.result.fields.recurrence) {
    for (const rule of EXCEPTION_RULES) {
      for (;;) {
        const clause = extractBoundedClause(ctx.working, rule.regex);
        if (!clause) {
          break;
        }

        ctx.working = normalizeWhitespace(`${ctx.working.slice(0, clause.start)} ${ctx.working.slice(clause.end)}`);
        pushException(clause.text, clause.start, clause.start + clause.text.length, rule.id, rule.note, 0.96);
      }
    }

    consume(
      'recurrence_start',
      new RegExp(`\\b(?:starting|beginning|from)\\s+${DATE_FRAGMENT}\\b`, 'i'),
      0.95,
      'extracted reminder recurrence start',
    );
    consume(
      'recurrence_window',
      /\bfor\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b/i,
      0.94,
      'extracted reminder recurrence window',
    );
    consume(
      'recurrence_end',
      new RegExp(`\\b(?:until|till|through|ending(?:\\s+on)?)\\s+${DATE_FRAGMENT}\\b`, 'i'),
      0.95,
      'extracted reminder recurrence end',
    );
  }

  if (!ctx.result.fields.context_hint) {
    const hint = ctx.working.match(TRAILING_CONTEXT_HINT_REGEX);
    if (hint && hint[1] && hint.index != null) {
      const recurrenceSpan = ctx.result.meta.spans.recurrence[0] || null;
      const key = hint[1].toLowerCase();
      const mapped = TIME_CONTEXTS[key];
      const rawTail = recurrenceSpan ? ctx.rawInput.slice(recurrenceSpan.end) : '';
      const rawTailHint = rawTail.match(/^\s*(?:at\s+)?(morning|noon|afternoon|evening|night)\b/i);
      if (mapped && rawTailHint?.[1]?.toLowerCase() === key) {
        ctx.result.fields.context_hint = key;
        ctx.state.explicitHour = mapped.hour;
        ctx.state.explicitMinute = mapped.minute;
        setFieldConfidence(ctx, 'context_hint', 0.72);
        addFieldSpan(ctx, 'context_hint', {
          start: hint.index ?? 0,
          end: (hint.index ?? 0) + hint[0].length,
          text: hint[0],
        });
        addDebugStep(ctx, {
          pass: 'scheduleDetailsPass',
          ruleId: 'schedule.context_hint',
          start: hint.index ?? 0,
          end: (hint.index ?? 0) + hint[0].length,
          text: hint[0],
          confidence: 0.72,
          note: 'captured recurring reminder time-of-day hint',
        });
        ctx.working = normalizeWhitespace(`${ctx.working.slice(0, hint.index)} ${ctx.working.slice(hint.index + hint[0].length)}`);
        syncMaskedInput(ctx);
      }
    }
  }
};
