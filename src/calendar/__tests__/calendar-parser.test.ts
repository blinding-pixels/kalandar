/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseEventInput } from '../index.ts';

const now = new Date('2026-03-27T10:00:00-04:00');
const timezone = 'America/New_York';

const parse = (input: string) => parseEventInput(input, { now, timezone });

describe('calendar parser accuracy regressions', () => {
  test('uses proper title case for small words in date-first sentence', () => {
    const result = parse('in 3 days finish the report');
    assert.equal(result.fields.title, 'Finish the Report');
    assert.equal(result.fields.date, '2026-03-30');
  });

  test('uses proper title case for small words in recurrence sentence', () => {
    const result = parse('check in with team every other week');
    assert.equal(result.fields.title, 'Check in with Team');
  });

  test('strips reminder lead phrase and title-cases event title', () => {
    const result = parse('remind me to call mom next month');
    assert.equal(result.fields.title, 'Call Mom');
    assert.equal(result.fields.date, '2026-04-27');
  });

  test('does not strip reminder prefix unless original input starts with it', () => {
    const stripped = parse('remind me to call mom tomorrow');
    assert.equal(stripped.fields.title, 'Call Mom');

    const preserved = parse('tomorrow remind me to call mom');
    assert.equal(preserved.fields.title, 'Remind Me to Call Mom');
    assert.equal(preserved.fields.date, '2026-03-28');
  });

  test('strips temporal context words like "morning" from final title', () => {
    const result = parse('buy groceries tomorrow morning');
    assert.equal(result.fields.title, 'Buy Groceries');
    assert.equal(result.fields.date, '2026-03-28');
  });

  test('keeps standalone "next" when it is part of the title', () => {
    const result = parse('plan next steps');
    assert.equal(result.fields.title, 'Plan Next Steps');
  });

  test('preserves standalone "start" in titles while parsing relative dates', () => {
    const result = parse('project start next week');
    assert.equal(result.fields.title, 'Project Start');
  });

  test('preserves standalone "night" while still parsing weekday dates', () => {
    const result = parse('game night friday');
    assert.equal(result.fields.title, 'Game Night');
    assert.equal(result.fields.date, '2026-03-27');
  });

  test('preserves standalone year tokens in titles', () => {
    const result = parse('budget 2026 review');
    assert.equal(result.fields.title, 'Budget 2026 Review');
  });

  test('preserves trailing phrasal prepositions when no temporal stripping occurred', () => {
    const result = parse('stand by');
    assert.equal(result.fields.title, 'Stand By');
  });

  test('preserves meaningful leading "in" title phrase', () => {
    const result = parse('in review');
    assert.equal(result.fields.title, 'In Review');
  });

  test('keeps standalone critical adjective in title', () => {
    const result = parse('critical design review next monday');
    assert.equal(result.fields.title, 'Critical Design Review');
    assert.equal(result.fields.date, '2026-03-30');
  });

  test('keeps trailing small words capitalized at title end', () => {
    const result = parse('what it is');
    assert.equal(result.fields.title, 'What It Is');
  });

  test('parses end-of-day and strips priority noise while preserving PR acronym', () => {
    const result = parse('high priority review PR by end of day');
    assert.equal(result.fields.date, '2026-03-27');
    assert.equal(result.fields.time, '17:00');
    assert.equal(result.fields.title, 'Review PR');
  });

  test('parses end-of-month and removes trailing temporal glue from title', () => {
    const result = parse('submit expenses end of month');
    assert.equal(result.fields.date, '2026-03-31');
    assert.equal(result.fields.title, 'Submit Expenses');
  });

  test('keeps critical adjectives like "urgent" in event titles', () => {
    const result = parse('urgent fix the login bug by friday for @mark');
    assert.equal(result.fields.title, 'Urgent Fix the Login Bug');
    assert.equal(result.fields.date, '2026-03-27');
  });
});
