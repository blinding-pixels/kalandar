/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseReminderInput } from '../index.ts';
import type { ReminderParseResult } from '../types.ts';

const now = new Date('2026-03-22T12:00:00-04:00');
const timezone = 'America/New_York';

const parse = (input: string) => parseReminderInput(input, { now, timezone });
const resolvedStart = (result: ReminderParseResult) => result.schedule?.start?.local ?? result.schedule?.start?.date ?? null;

describe('reminder parser time extraction', () => {
  test('parses "remind me to call mom tomorrow at 9am"', () => {
    const result = parse('remind me to call mom tomorrow at 9am');
    assert.equal(result.fields.remind_at, 'tomorrow at 9am');
    assert.equal(resolvedStart(result), '2026-03-23T09:00:00');
  });

  test('parses "don\'t forget dentist tonight at 7pm" using current tonight precedence', () => {
    const result = parse("don't forget dentist tonight at 7pm");
    assert.equal(result.fields.remind_at, 'tonight at 7pm');
    assert.equal(resolvedStart(result), '2026-03-22T19:00:00');
  });

  test('parses "ping me about the meeting in 2 hours" as now + 2h', () => {
    const result = parse('ping me about the meeting in 2 hours');
    assert.equal(result.fields.remind_at, 'in 2 hours');
    assert.equal(resolvedStart(result), '2026-03-22T14:00:00');
  });

  test('parses "remind me to call mom next month" with clean title', () => {
    const result = parse('remind me to call mom next month');
    assert.equal(result.fields.remind_at, 'next month');
    assert.equal(resolvedStart(result), '2026-04-01');
    assert.equal(result.fields.title, 'Call Mom');
  });

  test('parses "in 3 days finish the report" as a relative date with clean title', () => {
    const result = parse('in 3 days finish the report');
    assert.equal(result.fields.remind_at, 'in 3 days');
    assert.equal(resolvedStart(result), '2026-03-25');
    assert.equal(result.fields.title, 'Finish the Report');
  });

  test('does not infer recurrence from adjective use in "read daily digest tomorrow"', () => {
    const result = parse('read daily digest tomorrow');
    assert.equal(result.fields.recurrence, null);
    assert.equal(result.fields.remind_at, 'tomorrow');
    assert.equal(resolvedStart(result), '2026-03-23');
    assert.equal(result.fields.title, 'Read Daily Digest');
  });

  test('does not infer recurrence from adjective use in "buy monthly bus pass end of month"', () => {
    const result = parse('buy monthly bus pass end of month');
    assert.equal(result.fields.recurrence, null);
    assert.equal(result.fields.remind_at, 'end of month');
    assert.equal(resolvedStart(result), '2026-03-31');
    assert.equal(result.fields.title, 'Buy Monthly Bus Pass');
  });

  test('preserves email addresses when stripping temporal mentions', () => {
    const result = parse('email john@example.com tomorrow');
    assert.equal(result.fields.remind_at, 'tomorrow');
    assert.equal(resolvedStart(result), '2026-03-23');
    assert.equal(result.fields.title, 'Email John@example.com');
  });

  test('parses recurring monday reminder with explicit 8am time', () => {
    const result = parse('water plants every monday at 8am');
    assert.equal(result.fields.remind_at, '8am');
    assert.equal(resolvedStart(result), '2026-03-23T08:00:00');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 1,
      days: ['monday'],
    });
  });

  test('maps "end of day" to today at 17:00', () => {
    const result = parse('check email end of day');
    assert.equal(result.fields.remind_at, 'end of day');
    assert.equal(resolvedStart(result), '2026-03-22T17:00:00');
  });

  test('maps "end of week" to Friday at 17:00', () => {
    const result = parse('prepare recap end of week');
    assert.equal(result.fields.remind_at, 'end of week');
    assert.equal(resolvedStart(result), '2026-03-27');
  });

  test('maps "end of month" to the last day of month at 09:00 and cleans title', () => {
    const result = parse('submit expenses end of month');
    assert.equal(result.fields.remind_at, 'end of month');
    assert.equal(resolvedStart(result), '2026-03-31');
    assert.equal(result.fields.title, 'Submit Expenses');
  });

  test('consumes trailing punctuation after temporal phrase removal', () => {
    const result = parse('end of month, file taxes');
    assert.equal(result.fields.remind_at, 'end of month');
    assert.equal(resolvedStart(result), '2026-03-31');
    assert.equal(result.fields.title, 'File Taxes');
  });

  test('maps "end of month" to next month when current month target time has passed', () => {
    const rolloverNow = new Date('2026-03-31T10:00:00-04:00');
    const result = parseReminderInput('submit expenses end of month', { now: rolloverNow, timezone });
    assert.equal(result.fields.remind_at, 'end of month');
    assert.equal(resolvedStart(result), '2026-03-31');
    assert.equal(result.fields.title, 'Submit Expenses');
  });

  test('strips priority words and preserves uppercase acronyms in reminder titles', () => {
    const result = parse('high priority review PR by end of day');
    assert.equal(result.fields.remind_at, 'by end of day');
    assert.equal(resolvedStart(result), '2026-03-22T17:00:00');
    assert.equal(result.fields.title, 'Review PR');
  });

  test('strips priority, assignee mention, and dangling temporal prepositions from title', () => {
    const result = parse('urgent fix the login bug by friday for @mark');
    assert.equal(result.fields.remind_at, 'by friday');
    assert.equal(resolvedStart(result), '2026-03-27');
    assert.equal(result.fields.title, 'Fix the Login Bug');
  });

  test('preserves phrasal verb title after temporal token cleanup', () => {
    const result = parse('check in tomorrow');
    assert.equal(result.fields.remind_at, 'tomorrow');
    assert.equal(resolvedStart(result), '2026-03-23');
    assert.equal(result.fields.title, 'Check In');
  });

  test('strips reminder lead prefixes with smart apostrophes', () => {
    const result = parse('don\u2019t forget to call dentist');
    assert.equal(result.fields.title, 'Call Dentist');
  });

  test('parses ordinal date with default 09:00 time', () => {
    const result = parse('remind me on the 15th');
    assert.equal(result.fields.remind_at, 'on the 15th');
    assert.equal(resolvedStart(result), '2026-04-15');
  });

  test('normalizes "Bday" title to "Birthday"', () => {
    const result = parse('Bday');
    assert.equal(result.fields.title, 'Birthday');
  });

  test('interprets "call mom at 3" as 15:00 today via forward-date logic', () => {
    const result = parse('call mom at 3');
    assert.equal(result.fields.remind_at, 'at 3');
    assert.equal(resolvedStart(result), '2026-03-22T15:00:00');
  });

  test('extracts reminder recurrence window and end while using time-of-day hint for recurring reminders', () => {
    const result = parse('attach mailers to phil every wednesday night for 3 months till 9th august');
    assert.deepEqual(result.fields.recurrence, {
      frequency: 'weekly',
      interval: 1,
      days: ['wednesday'],
    });
    assert.equal(result.fields.recurrence_window, 'for 3 months');
    assert.equal(result.fields.recurrence_end, 'till 9th august');
    assert.equal(result.fields.context_hint, 'night');
    assert.equal(result.fields.remind_at, null);
    assert.equal(resolvedStart(result), '2026-03-25T20:00:00');
  });

  test('captures recurring "at night" as a context hint without leaving a dangling title token', () => {
    const result = parse('call mom every tuesday at night');

    assert.equal(result.fields.title, 'Call Mom');
    assert.equal(result.fields.context_hint, 'night');
    assert.equal(result.fields.remind_at, null);
    assert.equal(resolvedStart(result), '2026-03-24T20:00:00');
  });

  test('does not steal a title word like "night" into the schedule for recurring reminders', () => {
    const result = parse('movie night every friday');

    assert.equal(result.fields.title, 'Movie Night');
    assert.equal(result.fields.context_hint, null);
    assert.equal(result.fields.remind_at, null);
    assert.equal(resolvedStart(result), '2026-03-27');
  });
});
