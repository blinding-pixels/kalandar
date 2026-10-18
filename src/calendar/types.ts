import type { PreviewSchedule } from '../dates/index.ts';

export interface AlertField {
  offset_minutes: number;
}

export interface ContactSuggestion {
  text: string;
  source: 'title_with_phrase' | 'attendee_clause';
}

export type RecurrenceExceptionOrdinal = 'first' | 'second' | 'third' | 'last';
export type RecurrenceExceptionShiftDirection = 'following' | 'preceding';
export type RecurrenceExceptionWeekday =
  | 'monday'
  | 'tuesday'
  | 'wednesday'
  | 'thursday'
  | 'friday'
  | 'saturday'
  | 'sunday';

export interface HolidayRecurrenceExceptionCondition {
  type: 'holiday';
}

export interface WeekendRecurrenceExceptionCondition {
  type: 'weekend';
}

export interface OrdinalRecurrenceExceptionCondition {
  type: 'ordinal';
  which: RecurrenceExceptionOrdinal;
}

export interface SpecificDateRecurrenceExceptionCondition {
  type: 'specific_date';
  date: string;
}

export interface GenericRecurrenceExceptionCondition {
  type: string;
  [key: string]: unknown;
}

export type RecurrenceExceptionCondition =
  | HolidayRecurrenceExceptionCondition
  | WeekendRecurrenceExceptionCondition
  | OrdinalRecurrenceExceptionCondition
  | SpecificDateRecurrenceExceptionCondition
  | GenericRecurrenceExceptionCondition;

export interface SkipRecurrenceExceptionAction {
  type: 'skip';
}

export interface ShiftRecurrenceExceptionAction {
  type: 'shift';
  to: RecurrenceExceptionWeekday;
  direction: RecurrenceExceptionShiftDirection;
}

export interface ReplaceRecurrenceExceptionAction {
  type: 'replace';
  with: string;
}

export interface GenericRecurrenceExceptionAction {
  type: string;
  [key: string]: unknown;
}

export type RecurrenceExceptionAction =
  | SkipRecurrenceExceptionAction
  | ShiftRecurrenceExceptionAction
  | ReplaceRecurrenceExceptionAction
  | GenericRecurrenceExceptionAction;

export interface StructuredRecurrenceException {
  raw: string;
  condition: RecurrenceExceptionCondition | null;
  action: RecurrenceExceptionAction | null;
}

export interface EventFields {
  title: string | null;
  date: string | null;
  time: string | null;
  end_time: string | null;
  duration_minutes: number | null;
  location: string | null;
  recurrence: string | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_exception: StructuredRecurrenceException[];
  alerts: AlertField[] | null;
  attendees: string[] | null;
  contact_suggestions: ContactSuggestion[] | null;
}

export type ConfidenceFields = {
  title: number;
  date: number;
  time: number;
  end_time: number;
  duration_minutes: number;
  location: number;
  recurrence: number;
  recurrence_start: number;
  recurrence_end: number;
  recurrence_exception: number;
  alerts: number;
  attendees: number;
  contact_suggestions: number;
};

export interface FieldSpan {
  start: number;
  end: number;
  text: string;
  ruleId: string;
  confidence: number;
}

export type SpanFields = {
  title: FieldSpan[];
  date: FieldSpan[];
  time: FieldSpan[];
  end_time: FieldSpan[];
  duration_minutes: FieldSpan[];
  location: FieldSpan[];
  recurrence: FieldSpan[];
  recurrence_start: FieldSpan[];
  recurrence_end: FieldSpan[];
  recurrence_exception: FieldSpan[];
  alerts: FieldSpan[];
  attendees: FieldSpan[];
  contact_suggestions: FieldSpan[];
};

export interface DebugStep {
  pass: string;
  ruleId: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
  note: string;
}

export interface WarningSpan {
  start: number;
  end: number;
  text: string;
}

export interface ParseWarning {
  code: string;
  severity: 'info' | 'warning' | 'error';
  message: string;
  fieldHints: string[] | null;
  spans: WarningSpan[] | null;
  details: Record<string, unknown> | null;
}

export interface EventParseMeta {
  locale: string;
  timezone: string;
  runtime?: {
    mode: 'lightweight' | 'ml';
    suite_version?: string;
    artifact_id?: string;
  };
  confidence: ConfidenceFields;
  spans: SpanFields;
  cleanedInput: string;
  maskedInput: string;
  debugSteps: DebugStep[];
}

export interface EventParseResult {
  fields: EventFields;
  meta: EventParseMeta;
  warnings: ParseWarning[] | null;
  schedule?: PreviewSchedule | null;
}

export interface ParseOptions {
  now?: Date | string;
  timezone?: string;
  locale?: string;
  debug?: boolean;
}

export interface ParseContext {
  rawInput: string;
  cleanedInput: string;
  maskedInput: string;
  now: Date;
  options: {
    timezone: string;
    locale: string;
    debug: boolean;
  };
  result: EventParseResult;
}

export type ParsePass = (ctx: ParseContext) => void;

export type FieldName = keyof EventFields;

export type ScalarFieldName = Exclude<
  FieldName,
  'alerts' | 'attendees' | 'contact_suggestions' | 'recurrence_exception'
>;

export interface PassMatch {
  start: number;
  end: number;
  text: string;
  ruleId: string;
  confidence: number;
}
