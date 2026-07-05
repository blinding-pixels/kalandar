/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createTaskParser, parseTaskInput } from '../index.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('task parser object lightweight mode', () => {
  test('matches parseTaskInput for representative cases', async () => {
    const parser = await createTaskParser({ mode: 'lightweight' });
    const inputs = [
      'finish the board deck tomorrow at 3 pm',
      'Submit report to Sara every tuesday except the second tuesday of every month starting today till 24th August with Highpriorty by 10 AM',
      'urgent fix login for Project Atlas with @sam by friday, 30 minutes, in progress, tags #ops',
    ];

    for (const input of inputs) {
      const expected = parseTaskInput(input, { now, timezone });
      const actual = await parser.parse(input, { now, timezone });
      assert.deepEqual(actual, expected);
    }
  });
});
