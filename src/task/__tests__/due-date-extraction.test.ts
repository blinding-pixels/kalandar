/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseTaskInput } from '../index.ts';

const now = new Date('2026-03-22T12:00:00-04:00');
const timezone = 'America/New_York';

const parse = (input: string) => parseTaskInput(input, { now, timezone });

describe('task parser schedule extraction', () => {
  test('splits one-off date and time for "buy groceries today at 6pm"', () => {
    const result = parse('buy groceries today at 6pm');
    assert.equal(result.fields.due_date, 'today');
    assert.equal(result.fields.due_time, 'at 6pm');
    assert.equal(result.fields.recurrence, null);
  });

  test('keeps the date separate from the clock time for "call dentist at 3pm on tuesday"', () => {
    const result = parse('call dentist at 3pm on tuesday');
    assert.equal(result.fields.due_date, 'on tuesday');
    assert.equal(result.fields.due_time, 'at 3pm');
  });

  test('normalizes eod shorthand into due_date text', () => {
    const result = parse('send it by eod');
    assert.equal(result.fields.due_date, 'by end of day');
    assert.equal(result.fields.due_time, null);
  });

  test('extracts recurring spans without collapsing them into due_date', () => {
    const result = parse('Review backlog every other Tuesday starting next month by noon');
    assert.equal(result.fields.due_date, null);
    assert.equal(result.fields.due_time, 'by noon');
    assert.equal(result.fields.recurrence, 'every other Tuesday');
    assert.equal(result.fields.recurrence_start, 'starting next month');
  });

  test('extracts recurrence exception, start, end, and due_time in the reference recurring example', () => {
    const result = parse(
      'Submit report to Sara every tuesday except the second tuesday of every month starting today till 24th August with Highpriorty by 10 AM',
    );

    assert.equal(result.fields.recurrence, 'every tuesday');
    assert.deepEqual(result.fields.recurrence_exception, [
      {
        raw: 'except the second tuesday of every month',
        condition: { type: 'ordinal', which: 'second' },
        action: { type: 'skip' },
      },
    ]);
    assert.equal(result.fields.recurrence_start, 'starting today');
    assert.equal(result.fields.recurrence_end, 'till 24th August');
    assert.equal(result.fields.due_time, 'by 10 AM');
    assert.equal(result.fields.due_date, null);
  });

  test('extracts chained recurrence exceptions as separate structured entries', () => {
    const result = parse(
      "Submit report every Tuesday unless it's a holiday then the following Thursday starting next month but skip the first one by 10 AM",
    );

    assert.deepEqual(result.fields.recurrence_exception, [
      {
        raw: "unless it's a holiday then the following Thursday",
        condition: { type: 'holiday' },
        action: { type: 'shift', to: 'thursday', direction: 'following' },
      },
      {
        raw: 'but skip the first one',
        condition: { type: 'ordinal', which: 'first' },
        action: { type: 'skip' },
      },
    ]);
    assert.equal(result.fields.recurrence_start, 'starting next month');
    assert.equal(result.fields.due_time, 'by 10 AM');
  });
});
