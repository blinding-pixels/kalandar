import type { TaskParseOptions, TaskParseResult } from '../task/types.ts';
import type { TaskParserConfig } from '../task/index.ts';
import type { TaskDecompositionParserConfig, TaskDecompositionResult } from '../task-decomposition/types.ts';

export interface TaskPlanResult {
  task: TaskParseResult;
  decomposition: TaskDecompositionResult | null;
  meta: {
    parser: 'task_plan';
    task_runtime: TaskParseResult['meta']['runtime'];
    decomposition_enabled: boolean;
    decomposition_runtime?: TaskDecompositionResult['meta']['runtime'];
  };
}

export interface TaskPlanParseOptions extends TaskParseOptions {
  decompose?: boolean;
}

export interface TaskPlanParserInfo {
  parser: 'task_plan';
  task: {
    parser: 'task';
    mode: 'lightweight' | 'ml';
    suite_version?: string;
    artifact_id?: string;
  };
  decomposition?: {
    parser: 'task_decomposition';
    mode: 'lightweight' | 'ml';
    suite_version?: string;
    artifact_id?: string;
  };
}

export interface TaskPlanParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: TaskPlanParserInfo;
  parse(input: string, opts?: TaskPlanParseOptions): Promise<TaskPlanResult>;
  dispose(): Promise<void>;
}

export interface TaskPlanParserConfig {
  task?: TaskParserConfig;
  decomposition?: false | TaskDecompositionParserConfig;
}
