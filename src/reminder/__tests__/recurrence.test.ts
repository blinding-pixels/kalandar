/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseReminderInput } from '../index.ts';

const now = new Date('2026-03-22T12:00:00-04:00');
const timezone = 'America/New_York';

const parse = (input: string) => parseReminderInput(input, { now, timezone });

describe('reminder parser recurrence extraction', () => {
  test('extracts weekly recurrence for "water plants every monday"', () => {
    const result = parse('water plants every monday');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 1,
      days: ['monday'],
    });
  });

  test('extracts daily recurrence when "daily" appears at the start', () => {
    const result = parse('daily water plants');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'daily',
      interval: 1,
      days: null,
    });
    assert.equal(result.fields.title, 'Water Plants');
  });

  test('extracts monthly recurrence when "monthly" appears at the start', () => {
    const result = parse('monthly pay rent on the 1st');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'monthly',
      interval: 1,
      days: null,
    });
  });

  test('extracts every-other-week recurrence and keeps cleaned title', () => {
    const result = parse('check in with team every other week');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 2,
      days: null,
    });
    assert.equal(result.fields.title, 'Check in with Team');
  });

  test('strips orphaned leading filler after reminder + recurrence cleanup', () => {
    const result = parse('remind me every other day to stretch');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'daily',
      interval: 2,
      days: null,
    });
    assert.equal(result.fields.title, 'Stretch');
  });

  test('extracts interval recurrence for "every 3 days check inventory"', () => {
    const result = parse('every 3 days check inventory');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'daily',
      interval: 3,
      days: null,
    });
  });

  test('extracts multiple weekly days for "call grandma every sunday and wednesday"', () => {
    const result = parse('call grandma every sunday and wednesday');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 1,
      days: ['sunday', 'wednesday'],
    });
  });

  test('extracts yearly recurrence for "yearly on march 15"', () => {
    const result = parse('yearly on march 15');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'yearly',
      interval: 1,
      days: null,
    });
  });

  test('extracts reminder recurrence exception without shoving it into remind_at', () => {
    const result = parse('water plants every other saturday except the week of the 21st and add soil');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 2,
      days: ['saturday'],
    });
    assert.deepEqual(result.fields.recurrence_exception, [
      {
        raw: 'except the week of the 21st',
        condition: null,
        action: null,
      },
    ]);
  });

  test('extracts bounded nested reminder recurrence exceptions as separate structured entries', () => {
    const result = parse(
      "water plants every other saturday unless it's a holiday then the following Thursday but skip the first one",
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

  test('uses recurrence_start to move the first recurring remind_at forward', () => {
    const result = parse('lunch with sara every other tuesday starting next month');

    assert.equal(result.fields.recurrence_start, 'starting next month');
    assert.equal(result.fields.remind_at, '2026-04-07T09:00:00');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 2,
      days: ['tuesday'],
    });
  });

  test('preserves title tail after an exception clause instead of absorbing it into the exception', () => {
    const result = parse('review budget every monday unless it\'s a holiday and send note');

    assert.equal(result.fields.title, 'Review Budget and Send Note');
    assert.deepEqual(result.fields.recurrence_exception, [
      {
        raw: "unless it's a holiday",
        condition: { type: 'holiday' },
        action: { type: 'skip' },
      },
    ]);
  });
});
