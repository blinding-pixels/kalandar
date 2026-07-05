import type { TaskParseOptions } from '../task/types.ts';
import type { InferenceAdapter, LoadedMlArtifact, MlArtifactSource } from '../runtime/types.ts';

export type TaskDecompositionKind = 'none' | 'parallel' | 'paired_targets' | 'cross_product';
export type TaskDecompositionFieldState = 'explicit' | 'inherited' | 'unspecified';

export interface TaskDecompositionField {
  state: TaskDecompositionFieldState;
  value: string | null;
}

export interface TaskDecompositionPlanTask {
  title: string;
  due_date: TaskDecompositionField;
  due_time: TaskDecompositionField;
  project: TaskDecompositionField;
}

export interface TaskDecompositionChildTask extends TaskDecompositionPlanTask {
  target: string | null;
  subject: string | null;
  action: string | null;
}

export interface TaskDecompositionResult {
  should_suggest: boolean;
  kind: TaskDecompositionKind;
  confidence: number;
  parent: TaskDecompositionPlanTask | null;
  children: TaskDecompositionChildTask[];
  issues: string[];
  meta: {
    parser: 'task_decomposition';
    runtime: {
      mode: 'lightweight' | 'ml';
      suite_version?: string;
      artifact_id?: string;
    };
  };
}

export type TaskDecompositionParseOptions = TaskParseOptions;

export interface TaskDecompositionParserInfo {
  parser: 'task_decomposition';
  suite_version?: string;
  artifact_id?: string;
}

export interface TaskDecompositionParser {
  readonly mode: 'lightweight' | 'ml';
  readonly info: TaskDecompositionParserInfo;
  parse(input: string, opts?: TaskDecompositionParseOptions): Promise<TaskDecompositionResult>;
  dispose(): Promise<void>;
}

export type TaskDecompositionParserConfig =
  | { mode?: 'lightweight' }
  | {
      mode: 'ml';
      artifact: MlArtifactSource | LoadedMlArtifact<'task_decomposition'>;
      inference: InferenceAdapter;
    };
