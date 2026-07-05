/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createReminderParser, parseReminderInput } from '../index.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('reminder parser object lightweight mode', () => {
  test('matches parseReminderInput for representative cases', async () => {
    const parser = await createReminderParser({ mode: 'lightweight' });
    const inputs = [
      'remind me to call mom tomorrow at 9am',
      'attach mailers to phil every wednesday night for 3 months till 9th august',
      'water plants every other saturday except the week of the 21st and add soil',
    ];

    for (const input of inputs) {
      const expected = parseReminderInput(input, { now, timezone });
      const actual = await parser.parse(input, { now, timezone });
      assert.deepEqual(actual, expected);
    }
  });
});
