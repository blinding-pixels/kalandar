import type { StructuredRecurrenceException } from '../calendar/types.ts';
import type { DebugStep, FieldSpan, ParseWarning } from '../shared/types.ts';

export type TaskPriority = 'high' | 'medium' | 'low' | null;
export type TaskStatus = 'todo' | 'in progress' | 'done' | 'blocked' | 'waiting' | 'not started' | null;

export interface TaskFields {
  title: string;
  assignee: string[];
  project: string | null;
  tags: string[];
  priority: TaskPriority;
  status: TaskStatus;
  duration: string | null;
  due_date: string | null;
  due_time: string | null;
  recurrence: string | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_exception: StructuredRecurrenceException[];
}

export interface TaskParseMeta {
  runtime?: {
    mode: 'lightweight' | 'ml';
    suite_version?: string;
    artifact_id?: string;
  };
  confidence: {
    title: number;
    assignee: number;
    project: number;
    tags: number;
    priority: number;
    status: number;
    duration: number;
    due_date: number;
    due_time: number;
    recurrence: number;
    recurrence_start: number;
    recurrence_end: number;
    recurrence_exception: number;
  };
  spans: {
    title: FieldSpan[];
    assignee: FieldSpan[];
    project: FieldSpan[];
    tags: FieldSpan[];
    priority: FieldSpan[];
    status: FieldSpan[];
    duration: FieldSpan[];
    due_date: FieldSpan[];
    due_time: FieldSpan[];
    recurrence: FieldSpan[];
    recurrence_start: FieldSpan[];
    recurrence_end: FieldSpan[];
    recurrence_exception: FieldSpan[];
  };
  debugSteps: DebugStep[];
  maskedInput: string;
}

export interface TaskParseResult {
  fields: TaskFields;
  meta: TaskParseMeta;
  warnings: ParseWarning[] | null;
}

export interface TaskParseOptions {
  now?: Date | string;
  timezone?: string;
  knownAssignees?: string[];
  debug?: boolean;
}

export interface TaskParseContext {
  rawInput: string;
  working: string;
  maskedInput: string;
  now: Date;
  options: { timezone: string; knownAssignees?: string[]; debug: boolean };
  result: TaskParseResult;
  state: {
    recurring: boolean;
  };
}

export type TaskParsePass = (ctx: TaskParseContext) => void;
