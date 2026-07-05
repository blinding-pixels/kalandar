/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { afterEach, describe, test } from 'node:test';

import {
  classifyRecurrenceException,
  getRecurrenceExceptionResolver,
  parseEventInput,
  registerRecurrenceExceptionResolver,
  resetRecurrenceExceptionResolvers,
  resolveRecurrenceExceptionCondition,
} from '../index.ts';

const now = new Date('2026-03-27T10:00:00-04:00');
const timezone = 'America/New_York';

afterEach(() => {
  resetRecurrenceExceptionResolvers();
});

describe('recurrence exception classification', () => {
  test('classifies supported condition types', () => {
    assert.deepEqual(classifyRecurrenceException("unless it's a holiday", { now, timezone }), {
      raw: "unless it's a holiday",
      condition: { type: 'holiday' },
      action: { type: 'skip' },
    });

    assert.deepEqual(classifyRecurrenceException('except on weekends', { now, timezone }), {
      raw: 'except on weekends',
      condition: { type: 'weekend' },
      action: { type: 'skip' },
    });

    assert.deepEqual(classifyRecurrenceException('but not the second', { now, timezone }), {
      raw: 'but not the second',
      condition: { type: 'ordinal', which: 'second' },
      action: { type: 'skip' },
    });

    assert.deepEqual(classifyRecurrenceException('except on July 1st', { now, timezone }), {
      raw: 'except on July 1st',
      condition: { type: 'specific_date', date: '2026-07-01' },
      action: { type: 'skip' },
    });
  });

  test('classifies supported action types', () => {
    assert.deepEqual(classifyRecurrenceException("unless it's a holiday then the following Thursday", { now, timezone }), {
      raw: "unless it's a holiday then the following Thursday",
      condition: { type: 'holiday' },
      action: { type: 'shift', to: 'thursday', direction: 'following' },
    });

    assert.deepEqual(classifyRecurrenceException('except on holidays replace with async update', { now, timezone }), {
      raw: 'except on holidays replace with async update',
      condition: { type: 'holiday' },
      action: { type: 'replace', with: 'async update' },
    });

    assert.deepEqual(classifyRecurrenceException('move to the next Monday', { now, timezone }), {
      raw: 'move to the next Monday',
      condition: null,
      action: { type: 'shift', to: 'monday', direction: 'following' },
    });
  });

  test('leaves unclassifiable clauses as raw-only payloads', () => {
    assert.deepEqual(classifyRecurrenceException('except for the one after the first Monday in June', { now, timezone }), {
      raw: 'except for the one after the first Monday in June',
      condition: null,
      action: null,
    });
  });

  test('parser emits structured recurrence_exception arrays', () => {
    const result = parseEventInput(
      "Lunch with Sara every other Tuesday unless it's a holiday then the following Thursday starting next month but skip the first one",
      { now, timezone },
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
  });
});

describe('recurrence exception resolver registry', () => {
  test('registers and resolves a holiday resolver end to end', () => {
    registerRecurrenceExceptionResolver('holiday', (date) => ({
      matches: date.toISOString().startsWith('2026-12-25'),
      meta: { name: 'Christmas Day' },
    }));

    const resolver = getRecurrenceExceptionResolver('holiday');
    assert.equal(typeof resolver, 'function');

    const resolution = resolveRecurrenceExceptionCondition({ type: 'holiday' }, new Date('2026-12-25T12:00:00Z'));
    assert.deepEqual(resolution, {
      matches: true,
      meta: { name: 'Christmas Day' },
    });
  });

  test('falls back cleanly when no resolver is registered', () => {
    const resolution = resolveRecurrenceExceptionCondition({ type: 'holiday' }, new Date('2026-12-25T12:00:00Z'));
    assert.deepEqual(resolution, {
      matches: false,
      unresolvable: true,
    });
  });

  test('resolves built-in condition types without registry configuration', () => {
    assert.deepEqual(resolveRecurrenceExceptionCondition({ type: 'weekend' }, new Date('2026-03-28T12:00:00Z')), {
      matches: true,
    });

    assert.deepEqual(
      resolveRecurrenceExceptionCondition({ type: 'specific_date', date: '2026-07-01' }, new Date('2026-07-01T12:00:00Z')),
      {
        matches: true,
      },
    );
  });
});
