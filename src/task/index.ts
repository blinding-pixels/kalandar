import { finalizeTaskParseContext } from './finalize.ts';
import { parseTaskInputWithMl } from './ml.ts';
import { assigneePass } from './passes/assigneePass.ts';
import { priorityPass } from './passes/priorityPass.ts';
import { projectPass } from './passes/projectPass.ts';
import { schedulePass } from './passes/schedulePass.ts';
import { statusPass } from './passes/statusPass.ts';
import { tagsPass } from './passes/tagsPass.ts';
import { titlePass } from './passes/titlePass.ts';
import type { TaskParser, TaskParserConfig } from './ml.ts';
import type { TaskParseOptions, TaskParsePass, TaskParseResult } from './types.ts';
import { createTaskParseContext } from './utils.ts';
import { loadMlArtifact } from '../runtime/index.ts';
import type { LoadedMlArtifact, MlArtifactSource } from '../runtime/types.ts';

const PIPELINE: TaskParsePass[] = [
  priorityPass,
  statusPass,
  tagsPass,
  projectPass,
  assigneePass,
  schedulePass,
  titlePass,
];

const isLoadedMlArtifact = (artifact: unknown): artifact is LoadedMlArtifact =>
  Boolean(artifact && typeof artifact === 'object' && 'parser' in artifact && 'manifest' in artifact && 'tokenizer' in artifact);

export const parseTaskInput = (input: string, opts?: TaskParseOptions): TaskParseResult => {
  const ctx = createTaskParseContext(input, opts);
  ctx.result.meta.runtime = { mode: 'lightweight' };

  for (const pass of PIPELINE) {
    pass(ctx);
  }

  return finalizeTaskParseContext(ctx);
};

// Use createTaskParser({ mode: 'ml', artifact, inference }) for the explicit ML path.
export const createTaskParser = async (config: TaskParserConfig = { mode: 'lightweight' }): Promise<TaskParser> => {
  if (!config.mode || config.mode === 'lightweight') {
    return {
      mode: 'lightweight',
      info: {
        parser: 'task',
      },
      async parse(input: string, opts?: TaskParseOptions): Promise<TaskParseResult> {
        return parseTaskInput(input, opts);
      },
      async dispose(): Promise<void> {},
    };
  }

  let artifact: LoadedMlArtifact<'task'>;
  if (isLoadedMlArtifact(config.artifact)) {
    const loadedArtifact: LoadedMlArtifact = config.artifact;
    if (loadedArtifact.parser !== 'task') {
      throw new Error(`Artifact parser mismatch: expected task, received ${String(loadedArtifact.parser)}`);
    }
    artifact = loadedArtifact as LoadedMlArtifact<'task'>;
  } else {
    artifact = await loadMlArtifact(config.artifact, 'task');
  }
  await config.inference.init?.(artifact);

  return {
    mode: 'ml',
    info: {
      parser: 'task',
      suite_version: artifact.suite_version,
      artifact_id: artifact.artifact_id,
    },
    async parse(input: string, opts?: TaskParseOptions): Promise<TaskParseResult> {
      return parseTaskInputWithMl(input, opts, artifact, config.inference);
    },
    async dispose(): Promise<void> {
      await config.inference.dispose?.();
    },
  };
};

export type { TaskParser, TaskParserConfig } from './ml.ts';
export type {
  TaskFields,
  TaskParseContext,
  TaskParseMeta,
  TaskParseOptions,
  TaskParsePass,
  TaskParseResult,
  TaskPriority,
  TaskStatus,
} from './types.ts';
export type {
  CalendarBlockAdapterInput,
  CalendarBlockSpec,
  ReminderAdapterInput,
  ReminderDeliverySpec,
  SharedPlanningObject,
  SharedTemporalContract,
} from '../shared/planning.ts';
export {
  buildReminderDeliverySpec,
  buildTaskCalendarBlockSpec,
  buildTaskReminderDeliverySpec,
  taskResultToPlanningObject,
} from '../shared/planning.ts';
