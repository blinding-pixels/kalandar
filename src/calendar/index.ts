import { alertsPass } from './passes/alertsPass.ts';
import { attendeesPass } from './passes/attendeesPass.ts';
import { chronoPass } from './passes/chronoPass.ts';
import { durationPass } from './passes/durationPass.ts';
import { finalizeCalendarParseContext } from './finalize.ts';
import { locationPass } from './passes/locationPass.ts';
import { parseCalendarInputWithMl } from './ml.ts';
import { recurrencePass } from './passes/recurrencePass.ts';
import {
  classifyRecurrenceException,
  classifyRecurrenceExceptionAction,
  classifyRecurrenceExceptionCondition,
  configureRecurrenceExceptionResolvers,
  getRecurrenceExceptionResolver,
  registerRecurrenceExceptionResolver,
  resetRecurrenceExceptionResolvers,
  resolveRecurrenceExceptionCondition,
} from './recurrenceExceptions.ts';
import { titlePass } from './passes/titlePass.ts';
import type { CalendarParser, CalendarParserConfig } from './ml.ts';
import type { EventParseResult, ParseOptions } from './types.ts';
import { createParseContext, hasStructuredFields } from './utils.ts';
import { loadMlArtifact } from '../runtime/index.ts';
import type { LoadedMlArtifact, MlArtifactSource } from '../runtime/types.ts';

export type { CalendarParser, CalendarParserConfig } from './ml.ts';
export type {
  ContactSuggestion,
  DebugStep,
  EventFields,
  EventParseMeta,
  EventParseResult,
  FieldSpan,
  ParseOptions,
  ParseWarning,
  RecurrenceExceptionAction,
  RecurrenceExceptionCondition,
  StructuredRecurrenceException,
} from './types.ts';
export type {
  CalendarBlockAdapterInput,
  CalendarBlockSpec,
  ReminderDeliverySpec,
  SharedPlanningObject,
  SharedTemporalContract,
} from '../shared/planning.ts';
export { buildReminderDeliverySpec, eventResultToPlanningObject } from '../shared/planning.ts';
export {
  classifyRecurrenceException,
  classifyRecurrenceExceptionAction,
  classifyRecurrenceExceptionCondition,
  configureRecurrenceExceptionResolvers,
  getRecurrenceExceptionResolver,
  registerRecurrenceExceptionResolver,
  resetRecurrenceExceptionResolvers,
  resolveRecurrenceExceptionCondition,
};

const PIPELINE = [recurrencePass, alertsPass, durationPass, locationPass, attendeesPass, chronoPass, titlePass];

const isLoadedCalendarArtifact = (
  artifact: MlArtifactSource | LoadedMlArtifact<'calendar'>,
): artifact is LoadedMlArtifact<'calendar'> =>
  Boolean(
    artifact &&
      typeof artifact === 'object' &&
      'parser' in artifact &&
      'manifest' in artifact &&
      'tokenizer' in artifact &&
      artifact.parser === 'calendar',
  );

export const parseEventInput = (input: string, opts?: ParseOptions): EventParseResult => {
  const ctx = createParseContext(input, opts);
  ctx.result.meta.runtime = { mode: 'lightweight' };

  for (const pass of PIPELINE) {
    pass(ctx);
  }

  return finalizeCalendarParseContext(ctx);
};

export const createCalendarParser = async (
  config: CalendarParserConfig = { mode: 'lightweight' },
): Promise<CalendarParser> => {
  if (!config.mode || config.mode === 'lightweight') {
    return {
      mode: 'lightweight',
      info: {
        parser: 'calendar',
      },
      async parse(input: string, opts?: ParseOptions): Promise<EventParseResult> {
        return parseEventInput(input, opts);
      },
      async dispose(): Promise<void> {},
    };
  }

  if (config.artifact && typeof config.artifact === 'object' && 'parser' in config.artifact && config.artifact.parser !== 'calendar') {
    throw new Error(`Artifact parser mismatch: expected calendar, received ${String(config.artifact.parser)}`);
  }

  const artifact = isLoadedCalendarArtifact(config.artifact)
    ? config.artifact
    : await loadMlArtifact(config.artifact, 'calendar');
  await config.inference.init?.(artifact);

  return {
    mode: 'ml',
    info: {
      parser: 'calendar',
      suite_version: artifact.suite_version,
      artifact_id: artifact.artifact_id,
    },
    async parse(input: string, opts?: ParseOptions): Promise<EventParseResult> {
      return parseCalendarInputWithMl(input, opts, artifact, config.inference);
    },
    async dispose(): Promise<void> {
      await config.inference.dispose?.();
    },
  };
};

export const getStructuredParseConfidence = (result: EventParseResult): number => {
  const fields = result.meta.confidence;
  const weighted = [
    fields.date * 1.1,
    fields.time * 1,
    fields.end_time * 0.8,
    fields.duration_minutes * 0.8,
    fields.location * 0.7,
    fields.recurrence * 1,
    fields.recurrence_start * 0.7,
    fields.recurrence_end * 0.6,
    fields.recurrence_exception * 0.5,
    fields.alerts * 0.8,
    fields.attendees * 0.4,
    fields.contact_suggestions * 0.3,
  ];

  const sum = weighted.reduce((total, value) => total + value, 0);
  const max = 8.7;
  return Math.max(0, Math.min(1, sum / max));
};

export const shouldFallbackToPlainSearch = (result: EventParseResult): boolean => {
  if (!hasStructuredFields(result)) {
    return true;
  }
  return getStructuredParseConfidence(result) < 0.35;
};
