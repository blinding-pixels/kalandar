import { classifyIntent } from '../intent/index.ts';
import type { ParseWarning } from '../shared/types.ts';
import { DEFAULT_KNOWN_ASSIGNEES } from './constants.ts';
import type { TaskParseContext, TaskParseResult } from './types.ts';
import { getKnownAssigneeSet } from './utils.ts';

const buildTaskWarnings = (ctx: TaskParseContext): ParseWarning[] => {
  const warnings: ParseWarning[] = [];

  const intent = classifyIntent(ctx.rawInput);
  if (intent.intent === 'calendar') {
    warnings.push({
      code: 'intent_mismatch_calendar',
      severity: 'warning',
      message: 'Input looks more like a calendar item than a task.',
      fieldHints: ['title'],
      spans: ctx.result.meta.spans.title,
      details: {
        intent: intent.intent,
        topTwoGap: intent.meta.topTwoGap,
      },
    });
  } else if (intent.intent === 'reminder') {
    warnings.push({
      code: 'intent_mismatch_reminder',
      severity: 'warning',
      message: 'Input looks more like a reminder than a task.',
      fieldHints: ['title'],
      spans: ctx.result.meta.spans.title,
      details: {
        intent: intent.intent,
        topTwoGap: intent.meta.topTwoGap,
      },
    });
  } else if (intent.intent === 'none' && !intent.ambiguous) {
    warnings.push({
      code: 'intent_mismatch_none',
      severity: 'info',
      message: 'Input did not route cleanly to task or calendar.',
      fieldHints: ['title'],
      spans: ctx.result.meta.spans.title,
      details: {
        intent: intent.intent,
        topTwoGap: intent.meta.topTwoGap,
      },
    });
  }

  const knownSeed =
    ctx.options.knownAssignees && ctx.options.knownAssignees.length > 0
      ? ctx.options.knownAssignees
      : DEFAULT_KNOWN_ASSIGNEES;
  const knownSet = getKnownAssigneeSet(knownSeed);
  const unknownAssignees = ctx.result.fields.assignee.filter((hint) => {
    if (hint.startsWith('@') || hint.includes('@') || !ctx.options.knownAssignees?.length) {
      return false;
    }
    return !knownSet.has(hint.trim().toLowerCase());
  });

  if (unknownAssignees.length > 0) {
    warnings.push({
      code: 'unknown_assignee',
      severity: 'warning',
      message: `Unknown assignee: ${unknownAssignees.join(', ')}`,
      fieldHints: ['assignee'],
      spans: ctx.result.meta.spans.assignee,
      details: {
        unknownAssignees,
      },
    });
  }

  if (!ctx.result.fields.due_date && !ctx.result.fields.due_time && !ctx.result.fields.recurrence) {
    warnings.push({
      code: 'no_due_date',
      severity: 'info',
      message: 'No due date or recurrence extracted.',
      fieldHints: ['due_date', 'recurrence'],
      spans: [],
      details: {},
    });
  }

  return warnings;
};

export const finalizeTaskParseContext = (ctx: TaskParseContext): TaskParseResult => {
  const warnings = [...(ctx.result.warnings || []), ...buildTaskWarnings(ctx)];
  ctx.result.warnings = warnings.length > 0 ? warnings : null;
  ctx.result.meta.maskedInput = ctx.maskedInput;
  return ctx.result;
};
