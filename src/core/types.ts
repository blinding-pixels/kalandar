export interface FieldSpan {
  start: number;
  end: number;
  text: string;
}

export interface DebugStep {
  pass: string;
  ruleId: string;
  start: number;
  end: number;
  text: string;
  confidence: number;
  note: string;
}

export interface ParseWarning {
  code: string;
  severity: 'info' | 'warning' | 'error';
  message: string;
  fieldHints: string[];
  spans: FieldSpan[];
  details: Record<string, unknown>;
}
