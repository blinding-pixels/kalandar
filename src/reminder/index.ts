import { finalizeReminderParseContext } from './finalize.ts';
import { parseReminderInputWithMl } from './ml.ts';
import { prefixPass } from './passes/prefixPass.ts';
import { recurrencePass } from './passes/recurrencePass.ts';
import { scheduleDetailsPass } from './passes/scheduleDetailsPass.ts';
import { relativeTimePass } from './passes/relativeTimePass.ts';
import { absoluteTimePass } from './passes/absoluteTimePass.ts';
import { namedDatePass } from './passes/namedDatePass.ts';
import { chronoFallbackPass } from './passes/chronoFallbackPass.ts';
import { titlePass } from './passes/titlePass.ts';
import type { ReminderParser, ReminderParserConfig } from './ml.ts';
import type { ReminderParseOptions, ReminderParsePass, ReminderParseResult } from './types.ts';
import { createReminderParseContext } from './utils.ts';
import { loadMlArtifact } from '../runtime/index.ts';
import type { LoadedMlArtifact, MlArtifactSource } from '../runtime/types.ts';

const PIPELINE: ReminderParsePass[] = [
  prefixPass,
  recurrencePass,
  scheduleDetailsPass,
  relativeTimePass,
  absoluteTimePass,
  namedDatePass,
  chronoFallbackPass,
  titlePass,
];

const isLoadedReminderArtifact = (
  artifact: MlArtifactSource | LoadedMlArtifact<'reminder'>,
): artifact is LoadedMlArtifact<'reminder'> =>
  Boolean(
    artifact &&
      typeof artifact === 'object' &&
      'parser' in artifact &&
      'manifest' in artifact &&
      'tokenizer' in artifact &&
      artifact.parser === 'reminder',
  );

export const parseReminderInput = (input: string, opts?: ReminderParseOptions): ReminderParseResult => {
  const ctx = createReminderParseContext(input, opts);
  ctx.result.meta.runtime = { mode: 'lightweight' };

  for (const pass of PIPELINE) {
    pass(ctx);
  }

  return finalizeReminderParseContext(ctx);
};

export const createReminderParser = async (
  config: ReminderParserConfig = { mode: 'lightweight' },
): Promise<ReminderParser> => {
  if (!config.mode || config.mode === 'lightweight') {
    return {
      mode: 'lightweight',
      info: {
        parser: 'reminder',
      },
      async parse(input: string, opts?: ReminderParseOptions): Promise<ReminderParseResult> {
        return parseReminderInput(input, opts);
      },
      async dispose(): Promise<void> {},
    };
  }

  if (config.artifact && typeof config.artifact === 'object' && 'parser' in config.artifact && config.artifact.parser !== 'reminder') {
    throw new Error(`Artifact parser mismatch: expected reminder, received ${String(config.artifact.parser)}`);
  }

  const artifact = isLoadedReminderArtifact(config.artifact)
    ? config.artifact
    : await loadMlArtifact(config.artifact, 'reminder');
  await config.inference.init?.(artifact);

  return {
    mode: 'ml',
    info: {
      parser: 'reminder',
      suite_version: artifact.suite_version,
      artifact_id: artifact.artifact_id,
    },
    async parse(input: string, opts?: ReminderParseOptions): Promise<ReminderParseResult> {
      return parseReminderInputWithMl(input, opts, artifact, config.inference);
    },
    async dispose(): Promise<void> {
      await config.inference.dispose?.();
    },
  };
};

export type {
  ReminderFields,
  ReminderParseContext,
  ReminderParseMeta,
  ReminderParseOptions,
  ReminderParsePass,
  ReminderParseResult,
  ReminderRecurrence,
  ReminderRecurrenceFrequency,
} from './types.ts';
export type { ReminderParser, ReminderParserConfig } from './ml.ts';
export type {
  ReminderAdapterInput,
  ReminderDeliverySpec,
  SharedPlanningObject,
  SharedTemporalContract,
} from '../shared/planning.ts';
export { buildReminderDeliverySpec, reminderResultToPlanningObject } from '../shared/planning.ts';
