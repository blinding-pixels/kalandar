import assert from 'node:assert/strict';
import test from 'node:test';

import type { EventParseResult } from '../../calendar/types.ts';
import type { ReminderParseResult } from '../../reminder/types.ts';
import type { TaskParseResult } from '../../task/types.ts';
import {
  buildReminderDeliverySpec,
  buildTaskCalendarBlockSpec,
  buildTaskReminderDeliverySpec,
  eventResultToPlanningObject,
  reminderResultToPlanningObject,
  taskResultToPlanningObject,
} from '../planning.ts';

const buildTaskResult = (fields: Partial<TaskParseResult['fields']>): TaskParseResult => ({
  fields: {
    title: '',
    assignee: [],
    project: null,
    tags: [],
    priority: null,
    status: null,
    duration: null,
    due_date: null,
    due_time: null,
    recurrence: null,
    recurrence_start: null,
    recurrence_end: null,
    recurrence_exception: [],
    ...fields,
  },
  meta: {
    confidence: {
      title: 0,
      assignee: 0,
      project: 0,
      tags: 0,
      priority: 0,
      status: 0,
      duration: 0,
      due_date: 0,
      due_time: 0,
      recurrence: 0,
      recurrence_start: 0,
      recurrence_end: 0,
      recurrence_exception: 0,
    },
    spans: {
      title: [],
      assignee: [],
      project: [],
      tags: [],
      priority: [],
      status: [],
      duration: [],
      due_date: [],
      due_time: [],
      recurrence: [],
      recurrence_start: [],
      recurrence_end: [],
      recurrence_exception: [],
    },
    debugSteps: [],
    maskedInput: '',
  },
  warnings: null,
});

const buildReminderResult = (fields: Partial<ReminderParseResult['fields']>): ReminderParseResult => ({
  fields: {
    title: '',
    remind_at: null,
    recurrence: null,
    recurrence_start: null,
    recurrence_end: null,
    recurrence_exception: [],
    recurrence_window: null,
    location: null,
    context_hint: null,
    ...fields,
  },
  meta: {
    confidence: {
      title: 0,
      remind_at: 0,
      recurrence: 0,
      recurrence_start: 0,
      recurrence_end: 0,
      recurrence_exception: 0,
      recurrence_window: 0,
      location: 0,
      context_hint: 0,
    },
    spans: {
      title: [],
      remind_at: [],
      recurrence: [],
      recurrence_start: [],
      recurrence_end: [],
      recurrence_exception: [],
      recurrence_window: [],
      location: [],
      context_hint: [],
    },
    debugSteps: [],
    maskedInput: '',
  },
  warnings: null,
});

const buildEventResult = (fields: Partial<EventParseResult['fields']>): EventParseResult => ({
  fields: {
    title: null,
    date: null,
    time: null,
    end_time: null,
    duration_minutes: null,
    location: null,
    recurrence: null,
    recurrence_start: null,
    recurrence_end: null,
    recurrence_exception: [],
    alerts: null,
    attendees: null,
    contact_suggestions: null,
    ...fields,
  },
  meta: {
    locale: 'en-US',
    timezone: 'America/New_York',
    confidence: {
      title: 0,
      date: 0,
      time: 0,
      end_time: 0,
      duration_minutes: 0,
      location: 0,
      recurrence: 0,
      recurrence_start: 0,
      recurrence_end: 0,
      recurrence_exception: 0,
      alerts: 0,
      attendees: 0,
      contact_suggestions: 0,
    },
    spans: {
      title: [],
      date: [],
      time: [],
      end_time: [],
      duration_minutes: [],
      location: [],
      recurrence: [],
      recurrence_start: [],
      recurrence_end: [],
      recurrence_exception: [],
      alerts: [],
      attendees: [],
      contact_suggestions: [],
    },
    cleanedInput: '',
    maskedInput: '',
    debugSteps: [],
  },
  warnings: null,
});

test('task planning adapter preserves recurring schedule fields', () => {
  const planning = taskResultToPlanningObject(
    buildTaskResult({
      title: 'Submit report',
      assignee: ['Sara'],
      tags: ['#finance'],
      priority: 'high',
      status: 'todo',
      due_time: 'by 10 AM',
      recurrence: 'every Tuesday',
      recurrence_start: 'starting today',
      recurrence_end: 'till 24th August',
      recurrence_exception: [{ raw: 'except the second Tuesday of every month', condition: { type: 'ordinal', which: 'second' }, action: { type: 'skip' } }],
    }),
  );

  assert.equal(planning.kind, 'task');
  assert.equal(planning.title, 'Submit report');
  assert.deepEqual(planning.temporal.recurrence_exception, ['except the second Tuesday of every month']);
  assert.deepEqual(planning.contact_suggestions, [{ text: 'Sara', source: 'assignee' }]);
});

test('task reminder adapter can target reminders or notifications', () => {
  const spec = buildTaskReminderDeliverySpec(
    buildTaskResult({
      title: 'Submit report',
      assignee: ['Sara'],
      priority: 'high',
      status: 'todo',
      recurrence: 'every Tuesday',
      recurrence_start: 'starting today',
      recurrence_end: 'till 24th August',
      recurrence_exception: [{ raw: 'except the second Tuesday of every month', condition: { type: 'ordinal', which: 'second' }, action: { type: 'skip' } }],
    }),
    {
      destination: 'notifications',
      trigger_time: '10 AM',
    },
  );

  assert.equal(spec.destination, 'notifications');
  assert.equal(spec.trigger.trigger_time, '10 AM');
  assert.equal(spec.schedule.recurrence, 'every Tuesday');
  assert.equal(spec.schedule.recurrence_window, null);
});

test('task calendar block adapter reuses task schedule by default', () => {
  const block = buildTaskCalendarBlockSpec(
    buildTaskResult({
      title: 'Write outline',
      duration: '90 minutes',
      due_date: 'next Friday',
      due_time: '2 PM',
    }),
  );

  assert.equal(block.title, 'Write outline');
  assert.equal(block.date, 'next Friday');
  assert.equal(block.time, '2 PM');
  assert.equal(block.duration, '90 minutes');
});

test('reminder planning adapter preserves standalone reminder fields', () => {
  const planning = reminderResultToPlanningObject(
    buildReminderResult({
      title: 'Water plants',
      remind_at: '2026-04-18T09:00:00',
      recurrence: { frequency: 'weekly', interval: 2, days: ['saturday'] },
      recurrence_start: 'starting next month',
      recurrence_end: 'till 9th august',
      recurrence_exception: [{ raw: 'except the week of the 21st', condition: null, action: null }],
      recurrence_window: 'for 3 months',
      location: 'the patio',
      context_hint: 'before breakfast',
    }),
  );

  assert.equal(planning.kind, 'reminder');
  assert.equal(planning.temporal.time, '2026-04-18T09:00:00');
  assert.equal(planning.temporal.lead_time, 'before breakfast');
  assert.equal(planning.temporal.recurrence_window, 'for 3 months');
  assert.equal(planning.temporal.location, 'the patio');
  assert.deepEqual(planning.temporal.recurrence_exception, ['except the week of the 21st']);
});

test('event planning adapter carries recurrence exceptions and contact suggestions', () => {
  const planning = eventResultToPlanningObject(
    buildEventResult({
      title: 'Lunch with Sara',
      time: '12:00',
      end_time: '13:00',
      duration_minutes: 60,
      location: 'the cafe',
      recurrence: 'every other Tuesday',
      recurrence_start: 'starting next month',
      recurrence_exception: [{ raw: "unless it's a holiday then the following Thursday", condition: null, action: null }],
      alerts: [{ offset_minutes: 30 }],
      contact_suggestions: [{ text: 'Sara', source: 'title_with_phrase' }],
    }),
  );

  const spec = buildReminderDeliverySpec(planning, {
    destination: 'reminders',
    trigger_time: '11:00',
  });

  assert.equal(planning.kind, 'event');
  assert.deepEqual(planning.temporal.recurrence_exception, ["unless it's a holiday then the following Thursday"]);
  assert.deepEqual(planning.contact_suggestions, [{ text: 'Sara', source: 'title_with_phrase' }]);
  assert.equal(spec.trigger.trigger_time, '11:00');
});
