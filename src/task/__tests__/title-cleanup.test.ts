/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { parseTaskInput } from '../index.ts';

const now = new Date('2026-03-22T12:00:00-04:00');
const timezone = 'America/New_York';

const parseTitle = (input: string) => parseTaskInput(input, { now, timezone }).fields.title;

describe('task parser title cleanup', () => {
  test('strips one-off temporal tokens from the title', () => {
    assert.equal(parseTitle('buy groceries today at 7 pm'), 'buy groceries');
  });

  test('removes extracted priority and assignee phrases from the title', () => {
    assert.equal(parseTitle('urgent fix the login bug by friday for @mark'), 'fix the login bug');
  });

  test('preserves contextual "with <name>" title text when it is not an assignee clause', () => {
    assert.equal(parseTitle('meeting with sarah next monday at noon'), 'meeting with sarah');
  });

  test('preserves imperative title prefixes instead of trimming them away', () => {
    assert.equal(
      parseTitle(
        'finally finish the Q2 board deck draft for Project Atlas with Nina and Omar by the first business day next month, high priority, 90 minutes, in progress, tags #finance #board',
      ),
      'finally finish the Q2 board deck draft',
    );
  });

  test('strips reminder-style lead prefixes from task titles', () => {
    assert.equal(parseTitle('remind me to call mom next month'), 'call mom');
  });

  test('strips orphaned leading "to" after recurrence cleanup', () => {
    assert.equal(parseTitle('remind me every other day to stretch'), 'stretch');
  });

  test('preserves phrasal verb titles after temporal cleanup', () => {
    assert.equal(parseTitle('check in tomorrow'), 'check in');
  });

  test('strips shorthand date tokens after typo normalization', () => {
    assert.equal(parseTitle('tmrw morning call mom'), 'call mom');
  });

  test('strips filler and priority from terse noisy input', () => {
    assert.equal(parseTitle('pls fix the bug asap'), 'fix the bug');
  });
});
