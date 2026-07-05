import type { ParseContext } from '../types.ts';
import { addDebugStep, addFieldSpan, isSpanAvailable, maskMatch, setFieldConfidence } from '../utils.ts';

const PREFIXED_ATTENDEE_REGEX =
  /\b(?:with|invite|w\/)\s+([^,;]+?)(?=(?:\s+\b(?:every|at|in|on|from|to|for|starting|start|remind|alert|until|ending|except)\b|[,.;]|$))/gi;
const EMAIL_PATTERN = /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/gi;

const extractEmails = (input: string): RegExpMatchArray[] => {
  EMAIL_PATTERN.lastIndex = 0;
  return Array.from(input.matchAll(EMAIL_PATTERN));
};

const pushAttendee = (ctx: ParseContext, email: string): void => {
  const normalized = email.toLowerCase();
  const current = ctx.result.fields.attendees || [];
  if (!current.includes(normalized)) {
    current.push(normalized);
  }
  ctx.result.fields.attendees = current;
};

export const attendeesPass = (ctx: ParseContext): void => {
  const prefixedMatches = Array.from(ctx.maskedInput.matchAll(PREFIXED_ATTENDEE_REGEX));

  for (const prefixedMatch of prefixedMatches) {
    const fullText = prefixedMatch[0];
    const start = prefixedMatch.index ?? -1;
    const end = start + fullText.length;

    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    const emails = extractEmails(fullText);
    if (emails.length === 0) {
      continue;
    }

    for (const emailMatch of emails) {
      const emailText = emailMatch[0];
      const emailOffset = emailMatch.index ?? 0;
      const emailStart = start + emailOffset;
      const emailEnd = emailStart + emailText.length;

      pushAttendee(ctx, emailText);
      addFieldSpan(ctx, 'attendees', {
        start: emailStart,
        end: emailEnd,
        text: emailText,
        ruleId: 'attendees.prefixed_email',
        confidence: 0.95,
      });
      addDebugStep(ctx, {
        pass: 'attendees',
        ruleId: 'attendees.prefixed_email',
        start: emailStart,
        end: emailEnd,
        text: emailText,
        confidence: 0.95,
        note: 'email extracted from with/invite attendee clause',
      });
      setFieldConfidence(ctx, 'attendees', 0.95);
    }

    maskMatch(ctx, {
      start,
      end,
      text: fullText,
      ruleId: 'attendees.prefixed_clause',
      confidence: 0.88,
    });
  }

  // Fallback for bare emails not captured by attendee clauses.
  const fallbackEmails = extractEmails(ctx.maskedInput);
  for (const fallbackMatch of fallbackEmails) {
    const emailText = fallbackMatch[0];
    const start = fallbackMatch.index ?? -1;
    const end = start + emailText.length;

    if (!isSpanAvailable(ctx, start, end)) {
      continue;
    }

    pushAttendee(ctx, emailText);
    addFieldSpan(ctx, 'attendees', {
      start,
      end,
      text: emailText,
      ruleId: 'attendees.fallback_email',
      confidence: 0.55,
    });
    addDebugStep(ctx, {
      pass: 'attendees',
      ruleId: 'attendees.fallback_email',
      start,
      end,
      text: emailText,
      confidence: 0.55,
      note: 'fallback bare email attendee extraction',
    });
    setFieldConfidence(ctx, 'attendees', 0.55);
    maskMatch(ctx, {
      start,
      end,
      text: emailText,
      ruleId: 'attendees.fallback_email',
      confidence: 0.55,
    });
  }
};
