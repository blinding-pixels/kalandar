import type { EventParseResult } from '../calendar/types.ts';
import type { ReminderParseResult, ReminderRecurrence } from '../reminder/types.ts';
import type { TaskParseResult } from '../task/types.ts';

export type PlanningSourceKind = 'task' | 'reminder' | 'event';
export type DeliveryDestination = 'reminders' | 'notifications';

export interface SharedContactSuggestion {
  text: string;
  source: string;
}

export interface SharedTemporalContract {
  date: string | null;
  time: string | null;
  end_time: string | null;
  duration: string | number | null;
  lead_time: string | null;
  location: string | null;
  recurrence: string | ReminderRecurrence | null;
  recurrence_window: string | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_exception: string[];
}

export interface SharedPlanningObject {
  kind: PlanningSourceKind;
  title: string;
  temporal: SharedTemporalContract;
  contact_suggestions: SharedContactSuggestion[];
  task:
    | {
        assignee: string[];
        project: string | null;
        tags: string[];
        priority: string | null;
        status: string | null;
      }
    | null;
  reminder:
    | {
        remind_at: string | null;
        context_hint: string | null;
      }
    | null;
  event:
    | {
        alerts: { offset_minutes: number }[] | null;
        attendees: string[] | null;
      }
    | null;
}

export interface ReminderAdapterInput {
  destination: DeliveryDestination;
  trigger_time?: string | null;
  lead_time?: string | null;
  location?: string | null;
  title?: string | null;
}

export interface ReminderDeliverySpec {
  source_kind: PlanningSourceKind;
  destination: DeliveryDestination;
  title: string;
  anchor_title: string | null;
  trigger: {
    trigger_time: string | null;
    lead_time: string | null;
    location: string | null;
  };
  schedule: {
    recurrence: string | ReminderRecurrence | null;
    recurrence_window: string | null;
    recurrence_start: string | null;
    recurrence_end: string | null;
    recurrence_exception: string[];
  };
}

export interface CalendarBlockAdapterInput {
  date?: string | null;
  time?: string | null;
  end_time?: string | null;
  duration?: string | number | null;
  location?: string | null;
  title?: string | null;
}

export interface CalendarBlockSpec {
  source_kind: 'task';
  title: string;
  date: string | null;
  time: string | null;
  end_time: string | null;
  duration: string | number | null;
  location: string | null;
  recurrence: string | ReminderRecurrence | null;
  recurrence_window: string | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_exception: string[];
}

const toExceptionArray = (value: string | null | Array<{ raw: string }>): string[] => {
  if (!value) {
    return [];
  }
  if (typeof value === 'string') {
    return [value];
  }
  return value
    .map((entry) => entry.raw?.trim())
    .filter((entry): entry is string => Boolean(entry));
};

const trimOrEmpty = (value: string | null | undefined): string => (value || '').trim();

export const taskResultToPlanningObject = (result: TaskParseResult): SharedPlanningObject => ({
  kind: 'task',
  title: trimOrEmpty(result.fields.title),
  temporal: {
    date: result.fields.due_date,
    time: result.fields.due_time,
    end_time: null,
    duration: result.fields.duration,
    lead_time: null,
    location: null,
    recurrence: result.fields.recurrence,
    recurrence_window: null,
    recurrence_start: result.fields.recurrence_start,
    recurrence_end: result.fields.recurrence_end,
    recurrence_exception: toExceptionArray(result.fields.recurrence_exception),
  },
  contact_suggestions: result.fields.assignee.map((text) => ({ text, source: 'assignee' })),
  task: {
    assignee: result.fields.assignee,
    project: result.fields.project,
    tags: result.fields.tags,
    priority: result.fields.priority,
    status: result.fields.status,
  },
  reminder: null,
  event: null,
});

export const reminderResultToPlanningObject = (result: ReminderParseResult): SharedPlanningObject => ({
  kind: 'reminder',
  title: trimOrEmpty(result.fields.title),
  temporal: {
    date: null,
    time: result.fields.remind_at,
    end_time: null,
    duration: null,
    lead_time: result.fields.context_hint,
    location: result.fields.location,
    recurrence: result.fields.recurrence,
    recurrence_window: result.fields.recurrence_window,
    recurrence_start: result.fields.recurrence_start,
    recurrence_end: result.fields.recurrence_end,
    recurrence_exception: toExceptionArray(result.fields.recurrence_exception),
  },
  contact_suggestions: [],
  task: null,
  reminder: {
    remind_at: result.fields.remind_at,
    context_hint: result.fields.context_hint,
  },
  event: null,
});

export const eventResultToPlanningObject = (result: EventParseResult): SharedPlanningObject => ({
  kind: 'event',
  title: trimOrEmpty(result.fields.title),
  temporal: {
    date: result.fields.date,
    time: result.fields.time,
    end_time: result.fields.end_time,
    duration: result.fields.duration_minutes,
    lead_time: null,
    location: result.fields.location,
    recurrence: result.fields.recurrence,
    recurrence_window: null,
    recurrence_start: result.fields.recurrence_start,
    recurrence_end: result.fields.recurrence_end,
    recurrence_exception: result.fields.recurrence_exception.map((entry) => entry.raw),
  },
  contact_suggestions: (result.fields.contact_suggestions || []).map((entry) => ({
    text: entry.text,
    source: entry.source,
  })),
  task: null,
  reminder: null,
  event: {
    alerts: result.fields.alerts,
    attendees: result.fields.attendees,
  },
});

export const buildReminderDeliverySpec = (
  source: SharedPlanningObject,
  input: ReminderAdapterInput,
): ReminderDeliverySpec => ({
  source_kind: source.kind,
  destination: input.destination,
  title: trimOrEmpty(input.title) || source.title,
  anchor_title: source.title || null,
  trigger: {
    trigger_time: input.trigger_time ?? source.temporal.time,
    lead_time: input.lead_time ?? source.temporal.lead_time,
    location: input.location ?? source.temporal.location,
  },
  schedule: {
    recurrence: source.temporal.recurrence,
    recurrence_window: source.temporal.recurrence_window,
    recurrence_start: source.temporal.recurrence_start,
    recurrence_end: source.temporal.recurrence_end,
    recurrence_exception: [...source.temporal.recurrence_exception],
  },
});

export const buildTaskReminderDeliverySpec = (
  result: TaskParseResult,
  input: ReminderAdapterInput,
): ReminderDeliverySpec => buildReminderDeliverySpec(taskResultToPlanningObject(result), input);

export const buildTaskCalendarBlockSpec = (
  result: TaskParseResult,
  input: CalendarBlockAdapterInput = {},
): CalendarBlockSpec => {
  const source = taskResultToPlanningObject(result);
  return {
    source_kind: 'task',
    title: trimOrEmpty(input.title) || source.title,
    date: input.date ?? source.temporal.date,
    time: input.time ?? source.temporal.time,
    end_time: input.end_time ?? source.temporal.end_time,
    duration: input.duration ?? source.temporal.duration,
    location: input.location ?? source.temporal.location,
    recurrence: source.temporal.recurrence,
    recurrence_window: source.temporal.recurrence_window,
    recurrence_start: source.temporal.recurrence_start,
    recurrence_end: source.temporal.recurrence_end,
    recurrence_exception: [...source.temporal.recurrence_exception],
  };
};
