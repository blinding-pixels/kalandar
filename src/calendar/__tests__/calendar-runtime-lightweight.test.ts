/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createCalendarParser, parseEventInput } from '../index.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('calendar parser object lightweight mode', () => {
  test('matches parseEventInput for representative cases', async () => {
    const parser = await createCalendarParser({ mode: 'lightweight' });
    const inputs = [
      'Lunch with Sara every other Tuesday starting next month',
      "Lunch with Sara every other Tuesday unless it's a holiday then the following Thursday starting next month but skip the first one",
      'Design review tomorrow at 3 PM',
    ];

    for (const input of inputs) {
      const expected = parseEventInput(input, { now, timezone });
      const actual = await parser.parse(input, { now, timezone });
      assert.deepEqual(actual, expected);
    }
  });
});
