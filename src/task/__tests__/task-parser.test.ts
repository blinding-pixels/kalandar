/// <reference types="node" />

import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseTaskInput } from '../index.ts';
import type { TaskFields } from '../types.ts';

interface FixtureCase {
  name: string;
  text: string;
  expect: Partial<TaskFields>;
  notes?: string;
}

interface FixtureFile {
  meta: {
    now: string;
    timezone: string;
  };
  cases: FixtureCase[];
}

const fixturePath = new URL('../fixtures/simple-rule-parser-eval.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixturePath, 'utf8')) as FixtureFile;

const fieldsToCheck: Array<keyof TaskFields> = [
  'title',
  'assignee',
  'project',
  'tags',
  'priority',
  'status',
  'duration',
  'due_date',
  'due_time',
  'recurrence',
  'recurrence_start',
  'recurrence_end',
  'recurrence_exception',
];

test('parseTaskInput matches the curated rule-parser fixture and reports pass rates', () => {
  let casePassed = 0;
  const failures: string[] = [];
  const fieldCounts = Object.fromEntries(fieldsToCheck.map((field) => [field, { passed: 0, total: 0 }])) as Record<
    keyof TaskFields,
    { passed: number; total: number }
  >;

  for (const entry of fixture.cases) {
    const result = parseTaskInput(entry.text, {
      now: fixture.meta.now,
      timezone: fixture.meta.timezone,
    });

    let fullPass = true;
    for (const field of fieldsToCheck) {
      if (!(field in entry.expect)) {
        continue;
      }

      fieldCounts[field].total += 1;

      const expected = entry.expect[field];
      const actual = result.fields[field];
      let passed = false;

      if (Array.isArray(expected) && Array.isArray(actual)) {
        try {
          assert.deepEqual(actual, expected);
          passed = true;
        } catch {
          passed = false;
        }
      } else {
        passed = actual === expected;
      }

      if (passed) {
        fieldCounts[field].passed += 1;
      } else {
        fullPass = false;
      }
    }

    if (fullPass) {
      casePassed += 1;
      continue;
    }

    failures.push(
      [
        `${entry.name}: "${entry.text}"`,
        `expected=${JSON.stringify(entry.expect)}`,
        `received=${JSON.stringify(result.fields)}`,
        entry.notes ? `notes=${entry.notes}` : null,
      ]
        .filter(Boolean)
        .join(' | '),
    );
  }

  const caseRate = fixture.cases.length > 0 ? casePassed / fixture.cases.length : 0;
  console.log(`cases: ${(caseRate * 100).toFixed(1)}% (${casePassed}/${fixture.cases.length})`);

  for (const field of fieldsToCheck) {
    const counts = fieldCounts[field];
    if (counts.total === 0) {
      continue;
    }
    const rate = (counts.passed / counts.total) * 100;
    console.log(`${field}: ${rate.toFixed(1)}% (${counts.passed}/${counts.total})`);
  }

  assert.ok(
    caseRate >= 0.85,
    `curated rule-parser fixture pass rate ${caseRate.toFixed(2)} is below 0.85${
      failures.length > 0 ? ` | sample failures: ${failures.slice(0, 3).join(' || ')}` : ''
    }`,
  );
});
