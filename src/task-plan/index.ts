import { createTaskDecompositionParser, parseTaskDecompositionInput } from '../task-decomposition/index.ts';
import type { TaskDecompositionResult } from '../task-decomposition/types.ts';
import { createTaskParser, parseTaskInput } from '../task/index.ts';
import type { TaskParseOptions, TaskParseResult } from '../task/types.ts';
import type { TaskPlanParseOptions, TaskPlanParser, TaskPlanParserConfig, TaskPlanResult } from './types.ts';

const splitParseOptions = (opts?: TaskPlanParseOptions): { parserOpts: TaskParseOptions | undefined; decompose: boolean } => {
  if (!opts) {
    return { parserOpts: undefined, decompose: false };
  }

  const { decompose = false, ...parserOpts } = opts;
  return {
    parserOpts,
    decompose,
  };
};

const buildTaskPlanResult = (
  task: TaskParseResult,
  decomposition: TaskDecompositionResult | null,
  decompositionEnabled: boolean,
): TaskPlanResult => ({
  task,
  decomposition,
  meta: {
    parser: 'task_plan',
    task_runtime: task.meta.runtime,
    decomposition_enabled: decompositionEnabled,
    decomposition_runtime: decomposition?.meta.runtime,
  },
});

export const parseTaskPlanInput = (input: string, opts?: TaskPlanParseOptions): TaskPlanResult => {
  const { parserOpts, decompose } = splitParseOptions(opts);
  const task = parseTaskInput(input, parserOpts);
  const decomposition = decompose ? parseTaskDecompositionInput(input, parserOpts) : null;
  return buildTaskPlanResult(task, decomposition, decompose);
};

export const createTaskPlanParser = async (config: TaskPlanParserConfig = {}): Promise<TaskPlanParser> => {
  const taskParser = await createTaskParser(config.task || { mode: 'lightweight' });
  const decompositionParser = config.decomposition
    ? await createTaskDecompositionParser(config.decomposition)
    : null;

  return {
    mode:
      taskParser.mode === 'ml' || decompositionParser?.mode === 'ml'
        ? 'ml'
        : 'lightweight',
    info: {
      parser: 'task_plan',
      task: {
        parser: 'task',
        mode: taskParser.mode,
        suite_version: taskParser.info.suite_version,
        artifact_id: taskParser.info.artifact_id,
      },
      decomposition: decompositionParser
        ? {
            parser: 'task_decomposition',
            mode: decompositionParser.mode,
            suite_version: decompositionParser.info.suite_version,
            artifact_id: decompositionParser.info.artifact_id,
          }
        : undefined,
    },
    async parse(input: string, opts?: TaskPlanParseOptions): Promise<TaskPlanResult> {
      const { parserOpts, decompose } = splitParseOptions(opts);
      const task = await taskParser.parse(input, parserOpts);
      const decomposition = decompose && decompositionParser ? await decompositionParser.parse(input, parserOpts) : null;
      return buildTaskPlanResult(task, decomposition, decompose && Boolean(decompositionParser));
    },
    async dispose(): Promise<void> {
      await taskParser.dispose();
      await decompositionParser?.dispose();
    },
  };
};

export type { TaskPlanParseOptions, TaskPlanParser, TaskPlanParserConfig, TaskPlanParserInfo, TaskPlanResult } from './types.ts';
