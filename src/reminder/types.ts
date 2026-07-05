import type { DebugStep, FieldSpan, ParseWarning } from '../shared/types.ts';
import type { StructuredRecurrenceException } from '../calendar/types.ts';

export type ReminderRecurrenceFrequency = 'daily' | 'weekly' | 'monthly' | 'yearly';

export interface ReminderRecurrence {
  frequency: ReminderRecurrenceFrequency;
  interval: number;
  days: string[] | null;
}

export interface ReminderFields {
  title: string;
  remind_at: string | null;
  recurrence: ReminderRecurrence | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_exception: StructuredRecurrenceException[];
  recurrence_window: string | null;
  location: string | null;
  context_hint: string | null;
}

export interface ReminderParseMeta {
  runtime?: {
    mode: 'lightweight' | 'ml';
    suite_version?: string;
    artifact_id?: string;
  };
  confidence: {
    title: number;
    remind_at: number;
    recurrence: number;
    recurrence_start: number;
    recurrence_end: number;
    recurrence_exception: number;
    recurrence_window: number;
    location: number;
    context_hint: number;
  };
  spans: {
    title: FieldSpan[];
    remind_at: FieldSpan[];
    recurrence: FieldSpan[];
    recurrence_start: FieldSpan[];
    recurrence_end: FieldSpan[];
    recurrence_exception: FieldSpan[];
    recurrence_window: FieldSpan[];
    location: FieldSpan[];
    context_hint: FieldSpan[];
  };
  debugSteps: DebugStep[];
  maskedInput: string;
}

export interface ReminderParseResult {
  fields: ReminderFields;
  meta: ReminderParseMeta;
  warnings: ParseWarning[] | null;
}

export interface ReminderParseOptions {
  now?: Date | string;
  timezone?: string;
  debug?: boolean;
}

export interface ReminderParseContext {
  rawInput: string;
  working: string;
  maskedInput: string;
  now: Date;
  options: { timezone: string; debug: boolean };
  result: ReminderParseResult;
  state: {
    anchorDayOfMonth: number | null;
    anchorMonthDay: { month: number; day: number } | null;
    recurrenceStartDate: string | null;
    explicitHour: number | null;
    explicitMinute: number | null;
    ambiguousTime: boolean;
  };
}

export type ReminderParsePass = (ctx: ReminderParseContext) => void;
