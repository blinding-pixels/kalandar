/// <reference types="node" />

import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseReminderInput } from '../index.ts';
import type { ReminderFields } from '../types.ts';

interface FixtureCase {
  name: string;
  text: string;
  expect: Partial<ReminderFields>;
  notes?: string;
}

interface FixtureFile {
  meta: {
    now: string;
    timezone: string;
  };
  cases: FixtureCase[];
}

const fixturePath = new URL('../fixtures/rule-recurring-eval.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixturePath, 'utf8')) as FixtureFile;

test('parseReminderInput matches the curated recurring reminder fixture', () => {
  let passed = 0;

  for (const example of fixture.cases) {
    const result = parseReminderInput(example.text, {
      now: fixture.meta.now,
      timezone: fixture.meta.timezone,
    });

    for (const [fieldName, expected] of Object.entries(example.expect) as Array<[keyof ReminderFields, ReminderFields[keyof ReminderFields]]>) {
      assert.deepEqual(
        result.fields[fieldName],
        expected,
        `${example.name} "${example.text}" | field=${String(fieldName)} | notes=${example.notes || 'n/a'}`,
      );
    }

    passed += 1;
  }

  console.log(`curated reminder recurring eval: ${passed}/${fixture.cases.length} passing`);
});
