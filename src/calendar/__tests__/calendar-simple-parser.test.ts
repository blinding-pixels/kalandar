/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseEventInput } from '../index.ts';

const now = new Date('2026-03-27T10:00:00-04:00');
const timezone = 'America/New_York';

const parse = (input: string) => parseEventInput(input, { now, timezone });

describe('calendar lightweight parser recurrence contract', () => {
  test('parses a simple weekly recurrence with an end date and default one-hour duration', () => {
    const result = parse('Meeting with venue every Tuesday at 3 PM till July 1st');

    assert.equal(result.fields.title, 'Meeting with Venue');
    assert.equal(result.fields.recurrence, 'FREQ=WEEKLY;INTERVAL=1;BYDAY=TU');
    assert.equal(result.fields.recurrence_end, '2026-07-01');
    assert.equal(result.fields.time, '15:00');
    assert.equal(result.fields.end_time, '16:00');
    assert.equal(result.fields.duration_minutes, 60);
    assert.equal(result.fields.date, null);
  });

  test('surfaces recurrence_start for simple recurring phrases like starting next month', () => {
    const result = parse('Lunch with Sara every other Tuesday starting next month');

    assert.equal(result.fields.title, 'Lunch with Sara');
    assert.equal(result.fields.recurrence, 'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU');
    assert.equal(result.fields.recurrence_start, '2026-04-01');
    assert.equal(result.fields.recurrence_end, null);
    assert.equal(result.fields.date, null);
  });

  test('captures recurrence exceptions as raw clauses instead of leaking them into the title', () => {
    const result = parse(
      "Lunch with Sara every other Tuesday unless it's a holiday then the following Thursday starting next month but skip the first one",
    );

    assert.equal(result.fields.title, 'Lunch with Sara');
    assert.equal(result.fields.recurrence, 'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU');
    assert.equal(result.fields.recurrence_start, '2026-04-01');
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
    assert.equal(result.fields.date, null);
  });

  test('preserves with-phrases in the title and exposes contact suggestions separately', () => {
    const result = parse('Lunch with Sara every other Tuesday');

    assert.equal(result.fields.title, 'Lunch with Sara');
    assert.equal(result.fields.attendees, null);
    assert.deepEqual(result.fields.contact_suggestions, [{ text: 'Sara', source: 'title_with_phrase' }]);
  });

  test('defaults end time to one hour when only a start time exists', () => {
    const result = parse('Design review tomorrow at 3 PM');

    assert.equal(result.fields.time, '15:00');
    assert.equal(result.fields.end_time, '16:00');
    assert.equal(result.fields.duration_minutes, 60);
  });

  test('keeps monthly nth-weekday recurrence structure while leaving anchor-relative time unsupported', () => {
    const result = parse(
      'recurring catch-up every second Tuesday of the month starting next month, except for the one after the first Monday in June, from half past the hour of our usual Monday sync until three-quarters of an hour later.',
    );

    assert.equal(result.fields.title, 'Recurring Catch-up');
    assert.equal(result.fields.recurrence, 'FREQ=MONTHLY;INTERVAL=1;BYDAY=2TU');
    assert.equal(result.fields.recurrence_start, '2026-04-01');
    assert.deepEqual(result.fields.recurrence_exception, [
      {
        raw: 'except for the one after the first Monday in June',
        condition: null,
        action: null,
      },
    ]);
    assert.equal(result.fields.time, null);
    assert.equal(result.fields.end_time, null);
    assert.equal(result.fields.duration_minutes, null);
    assert.equal(result.fields.date, null);
  });

  test('masks unsupported speculative temporal clauses instead of hallucinating date or time from them', () => {
    const result = parse(
      'Book something next Friday night from 7ish to before 10, maybe 2 hours but not longer than 3, unless that runs too late then start earlier',
    );

    assert.equal(result.fields.title, 'Book Something');
    assert.equal(result.fields.date, '2026-04-03');
    assert.equal(result.fields.time, null);
    assert.equal(result.fields.end_time, null);
    assert.equal(result.fields.duration_minutes, null);
  });
});
