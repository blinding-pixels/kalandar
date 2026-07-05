import type { DebugStep, ParseWarning } from '../shared/types.ts';

export type IntentType = 'task' | 'calendar' | 'reminder' | 'none';

export interface IntentScores {
  task: number;
  calendar: number;
  reminder: number;
  none: number;
}

export interface IntentSignal {
  intent: IntentType;
  source: string;
  weight: number;
}

export interface IntentMeta {
  runtime: {
    mode: 'lightweight';
    model: 'tiny-linear';
    version: number;
  };
  debugSteps: DebugStep[];
  topTwoGap: number;
  signals: IntentSignal[];
}

export interface IntentResult {
  intent: IntentType;
  scores: IntentScores;
  ambiguous: boolean;
  meta: IntentMeta;
  warnings: ParseWarning[] | null;
}

export interface IntentOptions {
  threshold?: number;
}
